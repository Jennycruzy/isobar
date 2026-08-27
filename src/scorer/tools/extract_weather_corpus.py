#!/usr/bin/env python3
"""Convert an Explorer score capture into harness TSV files.

The Explorer's ``converted_answer`` is the text the intent scorer sees; the
raw ``miner_answer`` remains in the JSON capture for audit purposes.
"""

from __future__ import annotations

import argparse
import json
import re
from datetime import datetime, timedelta, timezone
from pathlib import Path


def one_line(value: object) -> str:
    return " ".join(str(value or "").split())


def replace_first_number(text: str) -> str:
    match = re.search(r"(?<![A-Za-z0-9.])-?\d+(?:\.\d+)?", text)
    if not match:
        return text
    token = match.group(0)
    value = float(token) + (10.0 if float(token) >= 0.0 else -10.0)
    replacement = f"{value:.1f}" if "." in token else str(int(value))
    return text[: match.start()] + replacement + text[match.end() :]


def swap_first_unit(text: str) -> str:
    patterns = [
        (r"°C", "°F"),
        (r"°F", "°C"),
        (r"\bCelsius\b", "Fahrenheit"),
        (r"\bFahrenheit\b", "Celsius"),
        (r"(?<=\d)\s*C\b", " F"),
        (r"(?<=\d)\s*F\b", " C"),
    ]
    for pattern, replacement in patterns:
        if re.search(pattern, text, flags=re.IGNORECASE):
            return re.sub(pattern, replacement, text, count=1, flags=re.IGNORECASE)
    return text


def invert_condition(text: str) -> str:
    wet = re.compile(
        r"\b(thunderstorms?|thundery|rain(?:fall)?|showers?|drizzle|snow|hail)\b",
        flags=re.IGNORECASE,
    )
    dry = re.compile(r"\b(clear(?:\s+sky)?|sunny|mainly clear)\b", flags=re.IGNORECASE)
    cloudy = re.compile(r"\b(overcast|partly cloudy|cloudy|clouds?)\b", flags=re.IGNORECASE)
    for pattern, replacement in (
        (wet, "clear"),
        (dry, "rain"),
        (cloudy, "rain"),
    ):
        if pattern.search(text):
            return pattern.sub(replacement, text, count=1)
    return text


ISO_TIMESTAMP = re.compile(
    r"\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?Z\b"
)


def stale_timestamp(text: str) -> str:
    match = ISO_TIMESTAMP.search(text)
    if not match:
        return text
    value = match.group(0)
    formats = ("%Y-%m-%dT%H:%M:%SZ", "%Y-%m-%dT%H:%M:%S.%fZ", "%Y-%m-%dT%H:%MZ")
    parsed = None
    for date_format in formats:
        try:
            parsed = datetime.strptime(value, date_format).replace(tzinfo=timezone.utc)
            break
        except ValueError:
            continue
    if parsed is None:
        return text
    shifted = parsed - timedelta(hours=2)
    if "." in value:
        rendered = shifted.strftime("%Y-%m-%dT%H:%M:%S.%fZ").rstrip("0").replace(".Z", "Z")
    elif value.count(":") == 1:
        rendered = shifted.strftime("%Y-%m-%dT%H:%MZ")
    else:
        rendered = shifted.strftime("%Y-%m-%dT%H:%M:%SZ")
    return text[: match.start()] + rendered + text[match.end() :]


def question_location(question: str) -> str | None:
    match = re.search(
        r"\b(?:in|at|for)\s+([A-Z][A-Za-z.'-]*(?:\s*,\s*[A-Z][A-Za-z.'-]*)?)",
        question,
    )
    if not match:
        return None
    return match.group(1).split(",", 1)[0].strip()


def wrong_city(question: str, text: str) -> str:
    city = question_location(question)
    if city and re.search(rf"\b{re.escape(city)}\b", text, flags=re.IGNORECASE):
        mutated = re.sub(rf"\b{re.escape(city)}\b", "Osaka", text, flags=re.IGNORECASE)
        return f"Weather in Osaka: {mutated}"
    return f"Osaka location: {text}"


def adversarial_answer(question: str, ground_truth: str, index: int) -> tuple[str, str]:
    mutations = (
        ("digit", lambda value: replace_first_number(value)),
        ("unit", lambda value: swap_first_unit(value)),
        ("condition", lambda value: invert_condition(value)),
        ("stale", lambda value: stale_timestamp(value)),
        ("location", lambda value: wrong_city(question, value)),
    )
    for offset in range(len(mutations)):
        label, mutate = mutations[(index + offset) % len(mutations)]
        candidate = mutate(ground_truth)
        if candidate != ground_truth:
            return label, candidate
    return "extra", f"{ground_truth} Unverified weather report."


def write_fixtures(path: Path, rows: list[dict[str, object]], start: int, end: int) -> None:
    output = ["# question\tground_truth\tgood_answer\tbad_answer"]
    for index, row in enumerate(rows[start:end], start=start):
        question = one_line(row.get("question", ""))
        ground_truth = one_line(row.get("ground_truth", ""))
        label, bad = adversarial_answer(question, ground_truth, index)
        output.append(
            "\t".join((question, ground_truth, ground_truth, one_line(bad)))
        )
    path.write_text("\n".join(output) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("capture", type=Path)
    parser.add_argument("--intent", default="WEATHER_CHECK")
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--scores", type=Path, required=True)
    parser.add_argument("--fixtures-fit", type=Path)
    parser.add_argument("--fixtures-holdout", type=Path)
    args = parser.parse_args()

    payload = json.loads(args.capture.read_text(encoding="utf-8"))
    rows = [row for row in payload["scores"] if row["intent_id"] == args.intent]
    if not rows:
        raise SystemExit(f"no rows found for {args.intent}")

    corpus = ["# id\tquestion\tground_truth\tminer_answer"]
    scores = [f"# Explorer {args.intent} score vector"]
    for row in rows:
        corpus.append(
            "\t".join(
                one_line(row.get(field, ""))
                for field in ("id", "question", "ground_truth", "converted_answer")
            )
        )
        scores.append(f"{row['id']}\t{row['score']}")

    args.corpus.write_text("\n".join(corpus) + "\n", encoding="utf-8")
    args.scores.write_text("\n".join(scores) + "\n", encoding="utf-8")
    if args.fixtures_fit or args.fixtures_holdout:
        split = (len(rows) * 78) // 100
        if args.fixtures_fit:
            write_fixtures(args.fixtures_fit, rows, 0, split)
        if args.fixtures_holdout:
            write_fixtures(args.fixtures_holdout, rows, split, len(rows))
    print(f"wrote {len(rows)} {args.intent} rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
