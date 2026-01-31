#!/usr/bin/env python3
"""
extract_salesforce_pageobjects.py

Extracts raw .utam.json page object definitions from the 
salesforce-pageobjects npm package.

Usage:
    python extract_salesforce_pageobjects.py [--output DIR] [--version VER]
    
Examples:
    python extract_salesforce_pageobjects.py
    python extract_salesforce_pageobjects.py --output ./pageobjects
    python extract_salesforce_pageobjects.py --version 10.0.2
"""

import argparse
import json
import subprocess
import sys
import tarfile
import tempfile
from datetime import datetime
from pathlib import Path


def run_npm_pack(package: str, cwd: Path) -> Path:
    """Run npm pack and return path to downloaded tarball."""
    result = subprocess.run(
        ["npm", "pack", package, "--silent"],
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"npm pack failed: {result.stderr}")
    
    # Find the tarball
    tarballs = list(cwd.glob("salesforce-pageobjects-*.tgz"))
    if not tarballs:
        raise RuntimeError("No tarball found after npm pack")
    return tarballs[0]


def extract_utam_json(tarball: Path, output_dir: Path) -> tuple[int, str]:
    """Extract .utam.json files from tarball, preserving directory structure."""
    output_dir.mkdir(parents=True, exist_ok=True)
    
    count = 0
    version = "unknown"
    
    with tarfile.open(tarball, "r:gz") as tar:
        for member in tar.getmembers():
            # Get version from package.json
            if member.name == "package/package.json":
                f = tar.extractfile(member)
                if f:
                    pkg = json.load(f)
                    version = pkg.get("version", "unknown")
                continue
            
            # Only extract .utam.json files from dist/
            if not member.name.endswith(".utam.json"):
                continue
            if not member.name.startswith("package/dist/"):
                continue
            
            # Calculate destination path (strip package/dist/ prefix)
            rel_path = member.name.replace("package/dist/", "", 1)
            dest_path = output_dir / rel_path
            
            # Create parent directories
            dest_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Extract file
            f = tar.extractfile(member)
            if f:
                dest_path.write_bytes(f.read())
                count += 1
    
    return count, version


def write_manifest(output_dir: Path, version: str, count: int):
    """Write manifest file with extraction metadata."""
    manifest: dict[str, str | int] = {
        "source": "salesforce-pageobjects",
        "version": version,
        "extracted": datetime.now().isoformat(),
        "file_count": count,
    }
    
    manifest_path = output_dir / "MANIFEST.json"
    manifest_path.write_text(json.dumps(manifest, indent=2))


def get_namespace_stats(output_dir: Path) -> dict[str, int]:
    """Get file counts per namespace."""
    stats: dict[str, int] = {}
    for item in output_dir.iterdir():
        if item.is_dir() and item.name != "__pycache__":
            count = len(list(item.rglob("*.utam.json")))
            if count > 0:
                stats[item.name] = count
    return dict(sorted(stats.items(), key=lambda x: -x[1]))


def main():
    parser = argparse.ArgumentParser(
        description="Extract UTAM page objects from salesforce-pageobjects npm package"
    )
    parser.add_argument(
        "--output", "-o",
        default="./salesforce-pageobjects",
        help="Output directory (default: ./salesforce-pageobjects)"
    )
    parser.add_argument(
        "--version", "-v",
        default="latest",
        help="Package version (default: latest)"
    )
    parser.add_argument(
        "--quiet", "-q",
        action="store_true",
        help="Suppress output"
    )
    args = parser.parse_args()
    
    output_dir = Path(args.output).resolve()
    package = f"salesforce-pageobjects@{args.version}"
    
    if not args.quiet:
        print(f"Downloading {package}...")
    
    # Create temp directory
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        
        # Download package
        try:
            tarball = run_npm_pack(package, temp_path)
        except RuntimeError as e:
            print(f"Error: {e}", file=sys.stderr)
            sys.exit(1)
        
        # Extract JSON files
        count, version = extract_utam_json(tarball, output_dir)
        
        # Write manifest
        write_manifest(output_dir, version, count)
    
    if not args.quiet:
        print(f"Extracted {count} page objects (v{version}) to {output_dir}/")
        print()
        
        # Show top namespaces
        stats = get_namespace_stats(output_dir)
        print("Top namespaces:")
        for ns, ns_count in list(stats.items())[:15]:
            print(f"  {ns:30} {ns_count:4} files")
        
        if len(stats) > 15:
            print(f"  ... and {len(stats) - 15} more namespaces")
        
        print()
        print(f"Manifest: {output_dir}/MANIFEST.json")


if __name__ == "__main__":
    main()