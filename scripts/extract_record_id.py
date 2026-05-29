"""Extract the Id of the first record from a Salesforce query JSON.

Usage:
    python extract_record_id.py [<json_file>]

Reads `result.records[0].Id` and prints it, or an empty string if there
are no records. Reads from a file argument when given, otherwise stdin.
"""

import json
import sys


def main() -> None:
    try:
        if len(sys.argv) >= 2:
            with open(sys.argv[1], "r", encoding="utf-8") as handle:
                data = json.load(handle)
        else:
            data = json.load(sys.stdin)
    except (OSError, json.JSONDecodeError) as exc:
        print(f"Warning: failed to read query JSON: {exc}", file=sys.stderr)
        print("")
        return

    records = data.get("result", {}).get("records", [])
    if records:
        print(records[0].get("Id", ""))
    else:
        print("")


if __name__ == "__main__":
    main()
