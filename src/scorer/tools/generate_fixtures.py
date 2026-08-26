#!/usr/bin/env python3
"""Generate deterministic fit and holdout answer-ranking fixtures."""

from pathlib import Path

FACTS = [
    ("What is the capital of France?", "The capital of France is Paris.", "Paris", "Berlin"),
    ("What does HTTP stand for?", "HTTP stands for Hypertext Transfer Protocol.", "Hypertext Transfer Protocol", "a database format"),
    ("How many days are in a leap year?", "A leap year has 366 days.", "366 days", "365 days"),
    ("What is the boiling point of water at sea level?", "Water boils at 100 degrees Celsius at sea level.", "100 degrees Celsius at sea level", "zero degrees Celsius"),
    ("What is Rust used for?", "Rust is a programming language focused on safety and performance.", "a programming language focused on safety and performance", "a markup language"),
    ("What is the largest planet in the Solar System?", "Jupiter is the largest planet in the Solar System.", "Jupiter", "Mars"),
    ("What does CPU mean?", "CPU means central processing unit.", "central processing unit", "computer power utility"),
    ("What is a hash function?", "A hash function maps input data to a fixed-size value.", "maps input data to a fixed-size value", "always encrypts data reversibly"),
    ("What is photosynthesis?", "Photosynthesis is the process by which plants use light to make chemical energy.", "the process by which plants use light to make chemical energy", "the process of freezing water"),
    ("What is a primary key?", "A primary key uniquely identifies a row in a database table.", "uniquely identifies a row in a database table", "duplicates a row identifier"),
    ("What is an IPv4 address?", "An IPv4 address is a 32-bit address written as four octets.", "a 32-bit address written as four octets", "a 128-bit address"),
    ("What is the purpose of TLS?", "TLS helps protect data in transit using encryption and authentication.", "protects data in transit using encryption and authentication", "compresses images only"),
    ("What is the chemical formula for water?", "The chemical formula for water is H2O.", "H2O", "CO2"),
    ("Which planet is known as the Red Planet?", "Mars is known as the Red Planet.", "Mars", "Venus"),
    ("What is the square root of 64?", "The square root of 64 is 8.", "8", "6"),
    ("What decimal number is binary 10?", "Binary 10 represents decimal 2.", "decimal 2", "decimal 10"),
    ("Which planet is third from the Sun?", "Earth is the third planet from the Sun.", "Earth", "Jupiter"),
    ("What is the approximate speed of light?", "The speed of light is approximately 299792 kilometers per second.", "approximately 299792 kilometers per second", "approximately 300 kilometers per second"),
    ("What kind of language is Python?", "Python is an interpreted programming language.", "an interpreted programming language", "a compiled markup language"),
    ("What shape does DNA commonly form?", "DNA commonly forms a double helix.", "a double helix", "a single flat sheet"),
    ("What is carbon's atomic number?", "Carbon has atomic number 6.", "6", "12"),
    ("What is the largest ocean on Earth?", "The Pacific Ocean is the largest ocean on Earth.", "the Pacific Ocean", "the Arctic Ocean"),
    ("How many days are in a week?", "There are 7 days in a week.", "7 days", "10 days"),
    ("What is the approximate value of pi?", "The approximate value of pi is 3.14159.", "3.14159", "2.71828"),
    ("What is the default port for HTTP?", "The default port for HTTP is 80.", "80", "443"),
    ("What is the default port for HTTPS?", "The default port for HTTPS is 443.", "443", "80"),
    ("What is Cargo in the Rust ecosystem?", "Cargo is Rust's package manager and build tool.", "Rust's package manager and build tool", "a database engine"),
    ("What does SQL stand for?", "SQL stands for Structured Query Language.", "Structured Query Language", "Simple Queue Logic"),
    ("What kind of memory is RAM?", "RAM is volatile computer memory.", "volatile computer memory", "permanent disk storage"),
    ("What does a CPU execute?", "A CPU executes computer instructions.", "computer instructions", "only image files"),
    ("What is the Sun?", "The Sun is a star.", "a star", "a planet"),
    ("What is the chemical formula for ozone?", "The chemical formula for ozone is O3.", "O3", "O2"),
    ("What is the SI unit of force?", "The SI unit of force is the newton.", "the newton", "the watt"),
    ("At what temperature does water freeze in Celsius?", "Water freezes at 0 degrees Celsius.", "0 degrees Celsius", "100 degrees Celsius"),
    ("What is the angle sum of a triangle?", "The angles in a triangle sum to 180 degrees.", "180 degrees", "360 degrees"),
    ("What does HTML stand for?", "HTML stands for Hypertext Markup Language.", "Hypertext Markup Language", "High Transfer Machine Logic"),
]

GOOD_TEMPLATES = (
    "{truth}",
    "The correct answer is {answer}.",
    "In short, {answer}.",
    "The relevant result is {answer}.",
)
BAD_TEMPLATES = (
    "The answer is {wrong}.",
    "In short, {wrong}.",
    "The relevant result is {wrong}.",
    "This is {wrong}.",
)


def rows(start: int, stop: int):
    for fact_index, (question, truth, answer, wrong) in enumerate(FACTS[start:stop], start):
        for variant, (good_template, bad_template) in enumerate(zip(GOOD_TEMPLATES, BAD_TEMPLATES)):
            yield (
                question,
                truth,
                good_template.format(truth=truth, answer=answer),
                bad_template.format(wrong=wrong),
            )


def write(path: Path, values) -> None:
    lines = ["# question\tground_truth\tgood_answer\tbad_answer"]
    lines.extend("\t".join(value) for value in values)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    fit = list(rows(0, 28))
    holdout = list(rows(28, len(FACTS)))
    write(root / "data/fixtures-fit.tsv", fit)
    write(root / "data/fixtures-holdout.tsv", holdout)
    write(root / "data/fixtures.tsv", fit + holdout)
    print(f"fit={len(fit)} holdout={len(holdout)} total={len(fit) + len(holdout)}")


if __name__ == "__main__":
    main()
