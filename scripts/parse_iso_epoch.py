"""Parse an ISO 8601 datetime string and print its Unix epoch timestamp."""

import sys
from datetime import datetime, timezone


def main() -> None:
    if len(sys.argv) < 2:
        print("")
        raise SystemExit(0)

    value = sys.argv[1]
    try:
        dt = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        print("")
        raise SystemExit(0)

    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)

    print(int(dt.timestamp()))


if __name__ == "__main__":
    main()
