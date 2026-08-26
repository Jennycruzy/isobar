#!/usr/bin/env python3
"""Select the strongest measured contrast point under agreement constraints."""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path


REQUIRED_COLUMNS = {
    "k",
    "centre",
    "margin",
    "agreement",
    "self_match",
    "ordering",
    "total",
    "ties",
}


def read_rows(path: Path) -> list[dict[str, object]]:
    with path.open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream)
        columns = set(reader.fieldnames or ())
        missing = REQUIRED_COLUMNS - columns
        if missing:
            raise ValueError(f"missing sweep columns: {', '.join(sorted(missing))}")
        rows = []
        for row in reader:
            rows.append(
                {
                    **row,
                    "k_value": float(row["k"]),
                    "centre_value": float(row["centre"]),
                    "margin_value": float(row["margin"]),
                    "agreement_value": float(row["agreement"]),
                    "self_match_value": float(row["self_match"]),
                    "ordering_value": int(row["ordering"]),
                    "total_value": int(row["total"]),
                    "ties_value": int(row["ties"]),
                }
            )
    if not rows:
        raise ValueError("sweep contains no rows")
    return rows


def pareto_frontier(rows: list[dict[str, object]]) -> list[dict[str, object]]:
    frontier = []
    best_margin = float("-inf")
    for row in sorted(rows, key=lambda item: (item["agreement_value"], item["margin_value"])):
        margin = row["margin_value"]
        if margin > best_margin:
            frontier.append(row)
            best_margin = margin
    return frontier


def clean_row(row: dict[str, object]) -> dict[str, object]:
    return {key: value for key, value in row.items() if not key.endswith("_value")}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sweep", type=Path, required=True)
    parser.add_argument("--min-agreement", type=float, default=0.95)
    parser.add_argument("--min-self-match", type=float, default=0.75)
    parser.add_argument("--min-ordering", type=int, default=0)
    parser.add_argument("--max-ties", type=int)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    try:
        rows = read_rows(args.sweep)
        eligible = [
            row
            for row in rows
            if row["agreement_value"] >= args.min_agreement
            and row["self_match_value"] >= args.min_self_match
            and row["ordering_value"] >= args.min_ordering
            and (args.max_ties is None or row["ties_value"] <= args.max_ties)
        ]
        if not eligible:
            raise ValueError("no sweep point satisfies the requested constraints")
        selected = max(
            eligible,
            key=lambda row: (
                row["margin_value"],
                row["agreement_value"],
                -row["ties_value"],
            ),
        )
        payload = {
            "constraints": {
                "min_agreement": args.min_agreement,
                "min_self_match": args.min_self_match,
                "min_ordering": args.min_ordering,
                "max_ties": args.max_ties,
            },
            "eligible_count": len(eligible),
            "selected": clean_row(selected),
            "pareto_frontier": [clean_row(row) for row in pareto_frontier(eligible)],
        }
    except (OSError, ValueError, TypeError) as error:
        print(f"could not select frontier: {error}", file=sys.stderr)
        return 2

    encoded = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    else:
        sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
