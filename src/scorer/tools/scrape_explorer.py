#!/usr/bin/env python3
"""Extract promotion evidence from an Explorer HTML/JSON response.

The Explorer has used both server-rendered and client-rendered Failed tabs.
This script deliberately stays read-only and accepts either a local capture or
an HTTPS URL. It extracts evidence conservatively; fields that are not visible
are left null rather than inferred.
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
from pathlib import Path
from urllib.request import Request, urlopen


REGISTRATION_RE = re.compile(r"#(\d+)\b")
INTENT_RE = re.compile(
    r"\b(ACADEMIC_SEARCH|AGENT_TASK|AI_TEXT_DETECTION|CHAT_COMPLETION|"
    r"CONTENT_EXTRACTION|CONTENT_MODERATION|IMAGE_VERIFICATION|"
    r"LANGUAGE_GENERATION|TASK_COMPLETION|WEATHER_CHECK|WEATHER_FORECAST|"
    r"WEB_SEARCH)\b"
)
NUMBER_RE = re.compile(r"(?<![\w.])(?:0?\.\d+|1(?:\.0+)?)\b")


def load_source(source: str) -> str:
    path = Path(source)
    if path.exists():
        return path.read_text(encoding="utf-8", errors="replace")
    request = Request(source, headers={"User-Agent": "isobar-scorer-explorer-report/0.1"})
    with urlopen(request, timeout=20) as response:
        return response.read().decode("utf-8", errors="replace")


def clean_text(value: str) -> str:
    value = re.sub(r"<script\b[^>]*>.*?</script>", " ", value, flags=re.I | re.S)
    value = re.sub(r"<style\b[^>]*>.*?</style>", " ", value, flags=re.I | re.S)
    value = re.sub(r"<[^>]+>", " ", value)
    value = html.unescape(value)
    return re.sub(r"\s+", " ", value).strip()


def records(source: str) -> list[dict[str, object]]:
    text = clean_text(source)
    lines = [line.strip() for line in re.split(r"[\n\r]+", text) if line.strip()]
    output: list[dict[str, object]] = []
    for index, line in enumerate(lines):
        lowered = line.lower()
        if not any(word in lowered for word in ("failed", "rejected", "agreement", "margin", "eval")):
            continue
        registration = REGISTRATION_RE.search(line)
        intent = INTENT_RE.search(line)
        numbers = [float(match.group(0)) for match in NUMBER_RE.finditer(line)]
        margin = None
        agreement = None
        margin_match = re.search(
            r"(?:margin|eval(?:uation)?|separation)\s*[:=]?\s*(0?\.\d+|1(?:\.0+)?)",
            line,
            flags=re.I,
        )
        agreement_match = re.search(
            r"agreement\s*[:=]?\s*(-?0?\.\d+|-?1(?:\.0+)?)",
            line,
            flags=re.I,
        )
        if margin_match:
            margin = float(margin_match.group(1))
        if agreement_match:
            agreement = float(agreement_match.group(1))
        output.append(
            {
                "line": index + 1,
                "registration_id": int(registration.group(1)) if registration else None,
                "intent": intent.group(1) if intent else None,
                "margin": margin,
                "agreement": agreement,
                "numbers": numbers,
                "text": line,
            }
        )
    return output


def wasm_api_result(payload: dict[str, object]) -> dict[str, object]:
    """Normalize the structured /api/wasm response without inferring fields."""
    intents = payload.get("intents")
    if not isinstance(intents, dict):
        raise ValueError("WASM API response has no intents object")

    normalized: list[dict[str, object]] = []
    summaries: dict[str, dict[str, object]] = {}
    for intent, value in sorted(intents.items()):
        if not isinstance(value, dict):
            continue
        champion = value.get("champion")
        champion = champion if isinstance(champion, dict) else {}
        champion_eval = champion.get("eval")
        champion_eval = champion_eval if isinstance(champion_eval, dict) else {}
        entries = value.get("entries")
        entries = entries if isinstance(entries, list) else []
        summary = {
            "champion_registration_id": champion.get("registration_id"),
            "champion_eval_score": champion.get("eval_score"),
            "champion_candidate_margin": champion_eval.get("candidate_margin"),
            "previous_champion_margin": champion_eval.get("champion_margin"),
            "entry_count": len(entries),
            "failed_count": sum(
                1
                for entry in entries
                if isinstance(entry, dict) and entry.get("activation_status") == "rejected"
            ),
            "pending_count": sum(
                1
                for entry in entries
                if isinstance(entry, dict) and entry.get("activation_status") == "pending"
            ),
        }
        summaries[str(intent)] = summary

        for entry in entries:
            if not isinstance(entry, dict):
                continue
            evaluation = entry.get("eval")
            evaluation = evaluation if isinstance(evaluation, dict) else {}
            spearman = evaluation.get("spearman")
            spearman = spearman if isinstance(spearman, dict) else {}
            normalized.append(
                {
                    "intent": intent,
                    "registration_id": entry.get("registration_id"),
                    "rank": entry.get("rank"),
                    "status": entry.get("activation_status"),
                    "is_champion": entry.get("is_champion", False),
                    "eval_score": entry.get("eval_score"),
                    "candidate_margin": evaluation.get("candidate_margin"),
                    "champion_margin": evaluation.get("champion_margin"),
                    "agreement": spearman.get(intent),
                    "worst_self_match": evaluation.get("worst_self_match"),
                    "candidate_wins": evaluation.get("candidate_wins"),
                    "comparable_cases": evaluation.get("comparable_cases"),
                    "rejection_reason": entry.get("rejection_reason"),
                    "registered_at": entry.get("registered_at"),
                    "updated_at": entry.get("updated_at"),
                }
            )

    return {
        "format": "wasm_api",
        "count": payload.get("count"),
        "intents": summaries,
        "records": normalized,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, help="saved HTML/JSON or HTTPS URL")
    parser.add_argument("--output", help="write JSON here instead of stdout")
    args = parser.parse_args()
    try:
        source_text = load_source(args.source)
        try:
            parsed = json.loads(source_text)
        except json.JSONDecodeError:
            parsed = None
        if isinstance(parsed, dict) and "intents" in parsed:
            result = wasm_api_result(parsed)
        else:
            result = {"format": "text", "records": records(source_text)}
        result["source"] = args.source
    except Exception as error:  # pragma: no cover - depends on source transport
        print(f"could not read source: {error}", file=sys.stderr)
        return 2
    payload = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        Path(args.output).write_text(payload, encoding="utf-8")
    else:
        sys.stdout.write(payload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
