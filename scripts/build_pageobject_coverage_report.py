#!/usr/bin/env python3
"""
Build a page-object-centered coverage report.

Inputs:
  * salesforce-pageobjects/coverage-inventory.json as the root PO universe
  * optional definition-coverage.json emitted by compiled_page_objects.rs
  * optional Allure result directories from the Salesforce live harness

Outputs:
  * pageobjects.json - machine-readable merged coverage state
  * pageobjects.csv  - review-friendly flat table
  * summary.md       - GitHub step summary / PR review summary
"""

from __future__ import annotations

import argparse
import csv
import json
from collections import Counter
from pathlib import Path
from typing import Any


STATUS_ORDER = {
    "broken": 5,
    "failed": 4,
    "unknown": 3,
    "skipped": 2,
    "passed": 1,
    "not_seen": 0,
    "not_reported": 0,
}


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        raise FileNotFoundError(f"required input does not exist: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid JSON in {path}: {exc}") from exc


def normalize_po_name(name: str) -> str:
    name = name.replace("\\", "/")
    if name.endswith(".utam.json"):
        name = name[: -len(".utam.json")]
    return name.strip("/")


def namespace_of(name: str) -> str:
    return name.split("/", 1)[0] if "/" in name else name


def parameter_map(result: dict[str, Any]) -> dict[str, str]:
    return {
        str(param.get("name")): str(param.get("value", ""))
        for param in result.get("parameters", [])
        if param.get("name") is not None
    }


def label_map(result: dict[str, Any]) -> dict[str, str]:
    labels: dict[str, str] = {}
    for label in result.get("labels", []):
        name = label.get("name")
        value = label.get("value")
        if name is not None and value is not None and str(name) not in labels:
            labels[str(name)] = str(value)
    return labels


def count_steps(result: dict[str, Any], prefix: str) -> Counter[str]:
    counts: Counter[str] = Counter()
    for step in result.get("steps", []):
        if str(step.get("name", "")).startswith(prefix):
            counts[str(step.get("status", "unknown"))] += 1
    return counts


def worst_status(statuses: list[str], empty: str = "not_seen") -> str:
    if not statuses:
        return empty
    return max(statuses, key=lambda status: STATUS_ORDER.get(status, STATUS_ORDER["unknown"]))


def make_inventory_row(name: str, entry: dict[str, Any]) -> dict[str, Any]:
    methods = entry.get("methods") or []
    parameterized = entry.get("parameterized_methods") or []
    return {
        "name": name,
        "namespace": entry.get("namespace") or namespace_of(name),
        "root": True,
        "standard": entry.get("standard"),
        "gated_reason": entry.get("gated_reason"),
        "root_selector": entry.get("root_selector"),
        "has_compiled_dts": bool(entry.get("has_compiled_dts")),
        "methods_defined": len(methods),
        "parameterized_methods": len(parameterized),
        "definition": {
            "status": "not_reported",
            "kind": None,
            "path": None,
            "error": None,
        },
        "live": {
            "status": "not_seen",
            "runs": [],
            "methods_passed": 0,
            "methods_failed": 0,
            "methods_skipped": 0,
            "elements_passed": 0,
            "elements_failed": 0,
            "elements_skipped": 0,
        },
        "behavioral": {
            "status": "not_seen",
            "checks": [],
        },
    }


def make_definition_only_row(name: str) -> dict[str, Any]:
    row = make_inventory_row(name, {"namespace": namespace_of(name)})
    row["root"] = False
    row["standard"] = None
    row["has_compiled_dts"] = False
    row["methods_defined"] = 0
    row["parameterized_methods"] = 0
    return row


def ensure_row(rows: dict[str, dict[str, Any]], name: str) -> dict[str, Any]:
    if name not in rows:
        rows[name] = make_definition_only_row(name)
    return rows[name]


def merge_definition_report(rows: dict[str, dict[str, Any]], path: Path) -> dict[str, Any]:
    report = load_json(path)
    results = report.get("results") or []
    for result in results:
        name = normalize_po_name(str(result.get("name", "")))
        if not name:
            continue
        row = ensure_row(rows, name)
        row["definition"] = {
            "status": result.get("status") or "unknown",
            "kind": result.get("kind"),
            "path": result.get("path"),
            "error": result.get("error"),
        }
    return {
        "path": str(path),
        "total": int(report.get("total") or len(results)),
        "passed": int(report.get("passed") or 0),
        "failed": int(report.get("failed") or 0),
    }


def iter_allure_results(dirs: list[Path]) -> list[Path]:
    files: list[Path] = []
    for result_dir in dirs:
        if not result_dir.exists():
            raise FileNotFoundError(f"Allure results directory does not exist: {result_dir}")
        if not result_dir.is_dir():
            raise NotADirectoryError(f"Allure results path is not a directory: {result_dir}")
        files.extend(sorted(result_dir.glob("*-result.json")))
    return files


def parse_context(full_name: str | None, params: dict[str, str]) -> str:
    if params.get("page_context"):
        return params["page_context"]
    if full_name and "salesforce_live::generic::" in full_name:
        rest = full_name.split("salesforce_live::generic::", 1)[1]
        return rest.split("::", 1)[0]
    return "unknown"


def merge_allure_results(rows: dict[str, dict[str, Any]], dirs: list[Path]) -> dict[str, Any]:
    result_files = iter_allure_results(dirs)
    live_summaries: list[dict[str, Any]] = []
    live_result_count = 0
    behavioral_count = 0

    for result_file in result_files:
        result = load_json(result_file)
        params = parameter_map(result)
        labels = label_map(result)
        full_name = result.get("fullName") or result.get("full_name")
        status = str(result.get("status", "unknown"))
        page_object = params.get("page_object")
        feature = labels.get("feature", "")

        if full_name and "salesforce_live::coverage::" in str(full_name):
            live_summaries.append(
                {
                    "name": result.get("name"),
                    "driver": params.get("driver", "unknown"),
                    "page_context": params.get("page_context", "unknown"),
                    "status": status,
                    "matched": params.get("matched"),
                    "loaded": params.get("loaded"),
                    "broken": params.get("broken"),
                    "out_of_scope": params.get("out_of_scope"),
                    "methods_passed": params.get("methods_passed"),
                    "methods_failed": params.get("methods_failed"),
                    "methods_skipped": params.get("methods_skipped"),
                    "elements_passed": params.get("elements_passed"),
                    "elements_failed": params.get("elements_failed"),
                    "elements_skipped": params.get("elements_skipped"),
                }
            )
            continue

        if not page_object:
            continue

        row = ensure_row(rows, normalize_po_name(page_object))
        driver = params.get("driver", "unknown")

        if feature == "Page Object Coverage" or (
            full_name and "salesforce_live::generic::" in str(full_name)
        ):
            live_result_count += 1
            method_counts = count_steps(result, "method: ")
            element_counts = count_steps(result, "element: ")
            run = {
                "driver": driver,
                "page_context": parse_context(str(full_name) if full_name else None, params),
                "status": status,
                "methods_passed": method_counts["passed"],
                "methods_failed": method_counts["failed"] + method_counts["broken"],
                "methods_skipped": method_counts["skipped"],
                "elements_passed": element_counts["passed"],
                "elements_failed": element_counts["failed"] + element_counts["broken"],
                "elements_skipped": element_counts["skipped"],
            }
            row["live"]["runs"].append(run)
            continue

        if feature == "Behavioral Assertions" or (
            full_name and "salesforce_live::behavioral::" in str(full_name)
        ):
            behavioral_count += 1
            row["behavioral"]["checks"].append(
                {
                    "name": result.get("name"),
                    "driver": driver,
                    "status": status,
                    "skip_reason": params.get("skip_reason"),
                }
            )

    for row in rows.values():
        live = row["live"]
        live["status"] = worst_status([run["status"] for run in live["runs"]])
        for run in live["runs"]:
            for key in (
                "methods_passed",
                "methods_failed",
                "methods_skipped",
                "elements_passed",
                "elements_failed",
                "elements_skipped",
            ):
                live[key] += int(run.get(key) or 0)
        behavioral = row["behavioral"]
        behavioral["status"] = worst_status([check["status"] for check in behavioral["checks"]])

    return {
        "result_files": len(result_files),
        "live_results": live_result_count,
        "behavioral_results": behavioral_count,
        "summaries": live_summaries,
    }


def summarize(rows: dict[str, dict[str, Any]], inventory: dict[str, Any], live: dict[str, Any]) -> dict[str, Any]:
    root_rows = [row for row in rows.values() if row["root"]]
    definition_statuses = Counter(row["definition"]["status"] for row in rows.values())
    live_statuses = Counter(row["live"]["status"] for row in root_rows)
    behavioral_statuses = Counter(row["behavioral"]["status"] for row in root_rows)
    driver_contexts: Counter[str] = Counter()
    for row in root_rows:
        for run in row["live"]["runs"]:
            driver_contexts[f"{run['driver']}:{run['page_context']}"] += 1

    return {
        "inventory": inventory.get("summary", {}),
        "rows": len(rows),
        "root_rows": len(root_rows),
        "definition_statuses": dict(sorted(definition_statuses.items())),
        "live_statuses": dict(sorted(live_statuses.items())),
        "behavioral_statuses": dict(sorted(behavioral_statuses.items())),
        "live_result_files": live["result_files"],
        "live_results": live["live_results"],
        "behavioral_results": live["behavioral_results"],
        "live_driver_contexts": dict(sorted(driver_contexts.items())),
    }


def write_json(path: Path, data: Any) -> None:
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    fieldnames = [
        "name",
        "namespace",
        "root",
        "standard",
        "gated_reason",
        "has_compiled_dts",
        "methods_defined",
        "parameterized_methods",
        "definition_status",
        "definition_kind",
        "live_status",
        "live_runs",
        "live_drivers",
        "live_contexts",
        "methods_passed",
        "methods_failed",
        "methods_skipped",
        "elements_passed",
        "elements_failed",
        "elements_skipped",
        "behavioral_status",
        "behavioral_checks",
    ]
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            live_runs = row["live"]["runs"]
            writer.writerow(
                {
                    "name": row["name"],
                    "namespace": row["namespace"],
                    "root": row["root"],
                    "standard": row["standard"],
                    "gated_reason": row["gated_reason"] or "",
                    "has_compiled_dts": row["has_compiled_dts"],
                    "methods_defined": row["methods_defined"],
                    "parameterized_methods": row["parameterized_methods"],
                    "definition_status": row["definition"]["status"],
                    "definition_kind": row["definition"]["kind"] or "",
                    "live_status": row["live"]["status"],
                    "live_runs": len(live_runs),
                    "live_drivers": ",".join(sorted({run["driver"] for run in live_runs})),
                    "live_contexts": ",".join(sorted({run["page_context"] for run in live_runs})),
                    "methods_passed": row["live"]["methods_passed"],
                    "methods_failed": row["live"]["methods_failed"],
                    "methods_skipped": row["live"]["methods_skipped"],
                    "elements_passed": row["live"]["elements_passed"],
                    "elements_failed": row["live"]["elements_failed"],
                    "elements_skipped": row["live"]["elements_skipped"],
                    "behavioral_status": row["behavioral"]["status"],
                    "behavioral_checks": len(row["behavioral"]["checks"]),
                }
            )


def markdown_table(rows: list[tuple[str, Any]]) -> list[str]:
    out = ["| Metric | Value |", "| --- | ---: |"]
    out.extend(f"| {name} | {value} |" for name, value in rows)
    return out


def write_markdown(path: Path, summary: dict[str, Any], rows: list[dict[str, Any]]) -> None:
    lines = ["## Page Object Coverage", ""]
    inventory = summary["inventory"]
    lines.extend(
        markdown_table(
            [
                ("Inventory root page objects", inventory.get("root_page_objects", 0)),
                ("Standard root page objects", inventory.get("standard", 0)),
                ("Gated root page objects", inventory.get("gated", 0)),
                ("Definition rows", summary["rows"]),
                ("Live result files", summary["live_result_files"]),
                ("Live page-object results", summary["live_results"]),
                ("Behavioral results", summary["behavioral_results"]),
            ]
        )
    )

    lines.extend(["", "### Definition Status", ""])
    lines.extend(markdown_table(list(summary["definition_statuses"].items()) or [("none", 0)]))

    lines.extend(["", "### Live Status", ""])
    lines.extend(markdown_table(list(summary["live_statuses"].items()) or [("none", 0)]))

    if summary["live_driver_contexts"]:
        lines.extend(["", "### Live Driver Contexts", ""])
        lines.extend(markdown_table(list(summary["live_driver_contexts"].items())))

    definition_failures = [
        row for row in rows if row["definition"]["status"] in {"failed", "broken", "unknown"}
    ]
    live_failures = [row for row in rows if row["live"]["status"] in {"failed", "broken"}]
    if definition_failures or live_failures:
        lines.extend(["", "### Review Queue", ""])
        lines.append("| Page object | Definition | Live | Reason |")
        lines.append("| --- | --- | --- | --- |")
        for row in (definition_failures + live_failures)[:25]:
            reason = row["definition"].get("error") or row.get("gated_reason") or ""
            lines.append(
                f"| `{row['name']}` | {row['definition']['status']} | {row['live']['status']} | {reason} |"
            )

    path.write_text("\n".join(lines) + "\n")


def build_report(args: argparse.Namespace) -> None:
    inventory = load_json(args.inventory)
    objects = inventory.get("objects") or {}
    rows = {
        name: make_inventory_row(name, entry)
        for name, entry in sorted(objects.items())
    }

    definition_reports = []
    for definition_path in args.definition or []:
        definition_reports.append(merge_definition_report(rows, definition_path))

    live = merge_allure_results(rows, args.allure_results or [])
    row_list = sorted(rows.values(), key=lambda row: row["name"])
    summary = summarize(rows, inventory, live)
    summary["definition_reports"] = definition_reports

    args.output_dir.mkdir(parents=True, exist_ok=True)
    write_json(
        args.output_dir / "pageobjects.json",
        {
            "schema_version": 1,
            "summary": summary,
            "page_objects": row_list,
        },
    )
    write_csv(args.output_dir / "pageobjects.csv", row_list)
    write_markdown(args.output_dir / "summary.md", summary, row_list)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--inventory",
        type=Path,
        default=Path("salesforce-pageobjects/coverage-inventory.json"),
        help="Coverage inventory JSON generated from the compiled JS client.",
    )
    parser.add_argument(
        "--definition",
        type=Path,
        action="append",
        help="definition-coverage.json emitted by compiled_page_objects.rs. Repeatable.",
    )
    parser.add_argument(
        "--allure-results",
        type=Path,
        action="append",
        help="Directory containing Allure *-result.json files. Repeatable.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("target/pageobject-coverage"),
        help="Directory for pageobjects.json, pageobjects.csv, and summary.md.",
    )
    return parser.parse_args()


def main() -> None:
    build_report(parse_args())


if __name__ == "__main__":
    main()