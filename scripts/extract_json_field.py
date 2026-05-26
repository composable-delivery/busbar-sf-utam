"""Extract a field from a JSON file's 'result' object.

Usage:
    python extract_json_field.py <field> <json_file> [<json_file> ...]

Prints the first non-empty value found for the given field across the
provided JSON files. If no value is found, prints an empty string.
"""

import json
import sys


def main() -> None:
    if len(sys.argv) < 3:
        print("Usage: extract_json_field.py <field> <json_file> [...]", file=sys.stderr)
        raise SystemExit(1)

    field = sys.argv[1]
    paths = sys.argv[2:]

    # Support comma-separated field names as fallback alternatives
    fields = [f.strip() for f in field.split(",")]

    for path in paths:
        try:
            with open(path, "r", encoding="utf-8") as handle:
                data = json.load(handle)
        except (OSError, json.JSONDecodeError) as exc:
            print(f"Warning: failed to read {path}: {exc}", file=sys.stderr)
            continue
        result = data.get("result", {})
        for f in fields:
            value = result.get(f, "")
            if value:
                print(value)
                return

    print("")


if __name__ == "__main__":
    main()
