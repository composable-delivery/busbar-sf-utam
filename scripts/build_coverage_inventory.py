#!/usr/bin/env python3
"""
build_coverage_inventory.py

Builds a coverage inventory for the Salesforce live tests by mining the
*compiled* UTAM JS client (the `salesforce-pageobjects` npm package).

The vendored `.utam.json` definitions describe selectors and element trees,
but the compiled `dist/**/pageObjects/*.d.ts` declarations are the authority
for how to *interact* with each page object:

  * method signatures with real argument NAMES + types
    (e.g. `getSearch(searchTerm: string)`, `getDockablePanel(apiName: string)`),
  * nullability, encoded in the return type (`Promise<... | null>` ==> the
    element/method is nullable and its absence is expected), and
  * a human description + root selector in the class JSDoc.

This script pairs each root page object's `.utam.json` with its compiled
`.d.ts`, extracts that interaction metadata, classifies each object as
`standard` (testable in a vanilla scratch org) vs `gated` (requires a
managed package or a feature/state a standard scratch org does not have),
and writes a single derived artifact: `coverage-inventory.json`.

We deliberately do NOT vendor the 1454 `.d.ts` files (huge, noisy diff).
We pull the package on demand with `npm pack` and commit only the small
derived inventory.

Usage:
    python scripts/build_coverage_inventory.py
    python scripts/build_coverage_inventory.py --version 11.0.0
    python scripts/build_coverage_inventory.py --output salesforce-pageobjects/coverage-inventory.json
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tarfile
import tempfile
from datetime import datetime, timezone
from pathlib import Path

# Root custom-element namespaces that belong to managed packages or
# features a vanilla scratch org does not have.  Discovery only tests an
# object when its root is actually present, so these mostly never match;
# we record them so the inventory can mark them out-of-scope explicitly
# (a reasoned skip, never a silent pass).  Keyed by the namespace prefix of
# the root selector's custom-element tag (the part before `_` or `-`).
GATED_TAG_NAMESPACES: dict[str, str] = {
    "devops_center": "DevOps Center managed package (not in a standard scratch org)",
    "clinical": "Health Cloud managed package",
    "industries": "an Industries cloud managed package",
    "mcontent": "CMS / managed content feature",
    "dxp": "Experience Cloud (Digital Experience) site",
    "experience": "Experience Cloud site",
    "wave": "CRM Analytics (Wave) feature",
    "analytics": "CRM Analytics feature",
    "flexipage": "",  # standard — kept here only as a no-op guard example
}
# Remove the no-op guard so it isn't treated as gated.
GATED_TAG_NAMESPACES.pop("flexipage", None)

# Specific objects (by `<namespace>/<name>` key) that are gated even though
# their root tag namespace looks standard, with the capability they need.
GATED_OBJECTS: dict[str, str] = {}


def run_npm_pack(package: str, cwd: Path) -> Path:
    result = subprocess.run(
        ["npm", "pack", package, "--silent"],
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"npm pack failed: {result.stderr}")
    tarballs = list(cwd.glob("salesforce-pageobjects-*.tgz"))
    if not tarballs:
        raise RuntimeError("No tarball found after npm pack")
    return tarballs[0]


# --- .d.ts parsing ---------------------------------------------------------

_CLASS_RE = re.compile(r"export default class\s+\w+\s+extends\s+\w+\s*\{(?P<body>.*)\}", re.DOTALL)
_METHOD_RE = re.compile(
    r"^\s{4}(?P<name>[A-Za-z_]\w*)\((?P<args>[^)]*)\)\s*:\s*Promise<(?P<ret>.+)>;\s*$",
    re.MULTILINE,
)
_SELECTOR_RE = re.compile(r"Selector:\s*(?P<sel>.+?)\.", re.DOTALL)


def parse_dts(text: str) -> dict | None:
    """Extract class description, root selector and methods from a .d.ts."""
    m = _CLASS_RE.search(text)
    if not m:
        return None
    body = m.group("body")

    # Class JSDoc: the /** ... */ block right before `export default class`.
    head = text[: m.start()]
    jsdoc = ""
    jb = list(re.finditer(r"/\*\*(.*?)\*/", head, re.DOTALL))
    if jb:
        jsdoc = jb[-1].group(1)
    selector = None
    sm = _SELECTOR_RE.search(jsdoc)
    if sm:
        selector = sm.group("sel").strip()
    desc_lines = [
        re.sub(r"^\s*\*\s?", "", ln).strip()
        for ln in jsdoc.splitlines()
        if "Selector:" not in ln and "generated from" not in ln and "@" not in ln
    ]
    description = " ".join(x for x in desc_lines if x)

    methods = []
    for mm in _METHOD_RE.finditer(body):
        name = mm.group("name")
        if name == "constructor":
            continue
        ret = mm.group("ret").strip()
        nullable = "| null" in ret or "|null" in ret
        args = []
        raw_args = mm.group("args").strip()
        if raw_args:
            for part in raw_args.split(","):
                if ":" in part:
                    an, at = part.split(":", 1)
                    args.append({"name": an.strip(), "type": at.strip()})
        methods.append(
            {
                "name": name,
                "args": args,
                "returns": ret,
                "nullable": nullable,
            }
        )
    return {"selector": selector, "description": description, "methods": methods}


def classify(po_key: str, root_tag: str | None) -> tuple[bool, str | None]:
    """Return (is_standard, gated_reason)."""
    if po_key in GATED_OBJECTS:
        return False, GATED_OBJECTS[po_key]
    if root_tag:
        # Namespace prefix is the part before the first `_` (LWC namespaces
        # like `devops_center`) — fall back to before `-` for tags such as
        # `devops_center-base-component`.
        ns = root_tag.split("-", 1)[0]
        ns_us = ns  # already includes the `_` namespace if present
        for gated_ns, reason in GATED_TAG_NAMESPACES.items():
            if ns_us == gated_ns or ns_us.startswith(gated_ns + "_"):
                return False, reason
    return True, None


def root_tag_of(selector_css: str | None) -> str | None:
    if not selector_css:
        return None
    m = re.match(r"\s*([A-Za-z][\w\-]*)", selector_css)
    return m.group(1) if m else None


def build(tarball: Path) -> dict:
    objects: dict[str, dict] = {}
    version = "unknown"
    with tarfile.open(tarball, "r:gz") as tar:
        members = tar.getmembers()
        # Index .d.ts by their dist-relative path for pairing.
        dts_by_path: dict[str, str] = {}
        utam_paths: list[str] = []
        for mem in members:
            n = mem.name
            if n == "package/package.json":
                f = tar.extractfile(mem)
                if f:
                    version = json.load(f).get("version", "unknown")
            elif n.endswith(".d.ts") and "/pageObjects/" in n and not n.endswith(".d.cts"):
                f = tar.extractfile(mem)
                if f:
                    dts_by_path[n] = f.read().decode("utf-8", "replace")
            elif n.endswith(".utam.json") and n.startswith("package/dist/"):
                utam_paths.append(n)

        for upath in utam_paths:
            rel = upath[len("package/dist/") :][: -len(".utam.json")]  # e.g. global/header
            # Only root page objects matter for discovery/coverage.
            f = tar.extractfile(next(m for m in members if m.name == upath))
            if not f:
                continue
            try:
                udef = json.load(f)
            except Exception:
                continue
            if not udef.get("root"):
                continue
            # Compiled classes live under `<namespace>/pageObjects/<rest>.d.ts`
            # — `pageObjects/` is inserted right after the top-level namespace
            # segment, e.g. `salesforceapp/record/actionBar.utam.json` pairs
            # with `salesforceapp/pageObjects/record/actionBar.d.ts`.
            parts = rel.split("/")
            ns = parts[0]
            rest = "/".join(parts[1:])
            dts_path = f"package/dist/{ns}/pageObjects/{rest}.d.ts"
            dts = dts_by_path.get(dts_path)
            parsed = parse_dts(dts) if dts else None

            root_css = (udef.get("selector") or {}).get("css")
            rtag = root_tag_of(root_css)
            is_std, reason = classify(rel, rtag)

            entry = {
                "name": rel,
                "namespace": ns,
                "root_selector": root_css,
                "standard": is_std,
                "gated_reason": reason,
                "description": (parsed or {}).get("description") if parsed else None,
                "has_compiled_dts": parsed is not None,
                "methods": (parsed or {}).get("methods", []) if parsed else [],
            }
            # Parameterized methods/elements (args present) need real values.
            entry["parameterized_methods"] = [
                m["name"] for m in entry["methods"] if m["args"]
            ]
            objects[rel] = entry

    standard = {k: v for k, v in objects.items() if v["standard"]}
    gated = {k: v for k, v in objects.items() if not v["standard"]}
    param_std = {k: v for k, v in standard.items() if v["parameterized_methods"]}
    return {
        "source": "salesforce-pageobjects (compiled UTAM JS client)",
        "version": version,
        "generated": datetime.now(timezone.utc).isoformat(),
        "summary": {
            "root_page_objects": len(objects),
            "standard": len(standard),
            "gated": len(gated),
            "standard_with_parameterized_calls": len(param_std),
            "with_compiled_dts": sum(1 for v in objects.values() if v["has_compiled_dts"]),
        },
        "objects": dict(sorted(objects.items())),
    }


def main() -> None:
    repo_root = Path(__file__).resolve().parent.parent
    manifest = repo_root / "salesforce-pageobjects" / "MANIFEST.json"
    default_version = "latest"
    if manifest.exists():
        try:
            default_version = json.loads(manifest.read_text()).get("version", "latest")
        except Exception:
            pass

    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--version", "-v", default=default_version)
    ap.add_argument(
        "--output",
        "-o",
        default=str(repo_root / "salesforce-pageobjects" / "coverage-inventory.json"),
    )
    ap.add_argument("--quiet", "-q", action="store_true")
    args = ap.parse_args()

    pkg = f"salesforce-pageobjects@{args.version}"
    if not args.quiet:
        print(f"Pulling compiled client {pkg} via npm pack ...")
    with tempfile.TemporaryDirectory() as td:
        tarball = run_npm_pack(pkg, Path(td))
        inv = build(tarball)

    out = Path(args.output)
    out.write_text(json.dumps(inv, indent=2) + "\n")
    if not args.quiet:
        s = inv["summary"]
        print(f"Wrote {out}")
        print(json.dumps(s, indent=2))


if __name__ == "__main__":
    main()
