//! Native evaluation harness for the deterministic scorer.

use isobar_scorer::scorer::{self, ScoringParams};
use std::cmp::Ordering;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

const DEFAULT_FIXTURES: &str = include_str!("../../data/fixtures.tsv");
const DEFAULT_BASELINE_FIXTURES: &str = include_str!("../../data/fixtures.baseline.tsv");
const DEFAULT_TRAFFIC: &str = include_str!("../../data/traffic.tsv");
const DEFAULT_CHAMPION_SCORES: &str = include_str!("../../data/traffic.baseline.tsv");
const MIN_SELF_MATCH: f32 = 0.75;
const MIN_MARGIN: f64 = 0.15;
const MIN_AGREEMENT: f64 = 0.60;
const DEFAULT_DETERMINISM_ITERATIONS: usize = 1000;
const DEFAULT_LATENCY_SAMPLES: usize = 1000;
const SWEEP_K_START: f32 = 4.0;
const SWEEP_K_STEP: f32 = 2.0;
const SWEEP_K_MAX: f32 = 32.0;
const SWEEP_C_START: f32 = 0.3;
const SWEEP_C_STEP: f32 = 0.1;
const SWEEP_C_MAX: f32 = 0.9;

#[derive(Clone)]
struct Fixture {
    question: String,
    ground_truth: String,
    good: String,
    bad: String,
}

#[derive(Clone)]
struct TrafficRow {
    id: String,
    question: String,
    ground_truth: String,
    answer: String,
}

struct ChampionScore {
    id: String,
    score: f32,
}

struct BaselineFixtureScore {
    self_match: f32,
    good: f32,
    bad: f32,
}

#[derive(Clone, Copy)]
struct Latency {
    p50_ns: u128,
    p99_ns: u128,
}

struct Metrics {
    self_match: f32,
    margin: f64,
    ordering: usize,
    fixture_total: usize,
    agreement: f64,
    ties: usize,
    duplicate_ties: usize,
    deterministic: bool,
    determinism_iterations: usize,
    latency: Latency,
    latency_samples: usize,
}

struct RawFixtureScores {
    baseline_self_match: f32,
    baseline_good: f32,
    baseline_bad: f32,
    candidate_self_match: f32,
    candidate_good: f32,
    candidate_bad: f32,
}

struct RawTrafficScores {
    champion: f32,
    candidate: f32,
}

struct RawCorpus {
    fixtures: Vec<RawFixtureScores>,
    traffic: Vec<RawTrafficScores>,
}

#[derive(Clone, Copy)]
struct ScoreKey<'a> {
    question: &'a str,
    ground_truth: &'a str,
    answer: &'a str,
}

fn cached_raw_score(question: &str, ground_truth: &str, answer: &str) -> f32 {
    scorer::raw_score(
        question.as_bytes(),
        ground_truth.as_bytes(),
        answer.as_bytes(),
    )
}

fn parse_fields(line: &str) -> Vec<&str> {
    if line.contains('\t') {
        line.split('\t').collect()
    } else {
        // The checked-in sample files remain readable when copied through
        // shells that turn tabs into the two-character "\\t" sequence.
        line.split("\\t").collect()
    }
}

fn parse_fixtures(contents: &str) -> Result<Vec<Fixture>, String> {
    let mut rows = Vec::new();
    for (line_number, line) in contents.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = parse_fields(line);
        if fields.len() != 4 {
            return Err(format!(
                "fixture line {} has {} fields",
                line_number + 1,
                fields.len()
            ));
        }
        rows.push(Fixture {
            question: fields[0].to_owned(),
            ground_truth: fields[1].to_owned(),
            good: fields[2].to_owned(),
            bad: fields[3].to_owned(),
        });
    }
    if rows.is_empty() {
        return Err("fixture set is empty".to_owned());
    }
    Ok(rows)
}

fn parse_baseline_fixtures(contents: &str) -> Result<Vec<BaselineFixtureScore>, String> {
    let mut rows = Vec::new();
    for (line_number, line) in contents.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = parse_fields(line);
        if fields.len() != 4 {
            return Err(format!(
                "baseline fixture line {} has {} fields",
                line_number + 1,
                fields.len()
            ));
        }
        let parse_score = |field: &str, name: &str| {
            let score = field.parse::<f32>().map_err(|error| {
                format!("baseline fixture line {} {name}: {error}", line_number + 1)
            })?;
            if !score.is_finite() || !(0.0..=1.0).contains(&score) {
                return Err(format!(
                    "baseline fixture line {} {name} is outside [0,1]",
                    line_number + 1
                ));
            }
            Ok(score)
        };
        rows.push(BaselineFixtureScore {
            self_match: parse_score(fields[1], "self-match")?,
            good: parse_score(fields[2], "good score")?,
            bad: parse_score(fields[3], "bad score")?,
        });
    }
    if rows.is_empty() {
        return Err("baseline fixture score set is empty".to_owned());
    }
    Ok(rows)
}

fn parse_traffic(contents: &str) -> Result<Vec<TrafficRow>, String> {
    let mut rows = Vec::new();
    for (line_number, line) in contents.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = parse_fields(line);
        if fields.len() != 4 {
            return Err(format!(
                "traffic line {} has {} fields",
                line_number + 1,
                fields.len()
            ));
        }
        rows.push(TrafficRow {
            id: fields[0].to_owned(),
            question: fields[1].to_owned(),
            ground_truth: fields[2].to_owned(),
            answer: fields[3].to_owned(),
        });
    }
    Ok(rows)
}

fn corpus_fixtures(rows: &[TrafficRow]) -> (Vec<Fixture>, Vec<BaselineFixtureScore>) {
    let fixtures = rows
        .iter()
        .map(|row| Fixture {
            question: row.question.clone(),
            ground_truth: row.ground_truth.clone(),
            good: row.ground_truth.clone(),
            bad: row.answer.clone(),
        })
        .collect::<Vec<_>>();
    let baseline = fixtures
        .iter()
        .map(|_| BaselineFixtureScore {
            self_match: 1.0,
            good: 1.0,
            bad: 0.0,
        })
        .collect::<Vec<_>>();
    (fixtures, baseline)
}

fn parse_champion_scores(contents: &str) -> Result<Vec<ChampionScore>, String> {
    let mut rows = Vec::new();
    for (line_number, line) in contents.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = parse_fields(line);
        if fields.len() != 2 {
            return Err(format!(
                "champion score line {} has {} fields",
                line_number + 1,
                fields.len()
            ));
        }
        let score = fields[1].parse::<f32>().map_err(|error| {
            format!(
                "champion score line {} is invalid: {error}",
                line_number + 1
            )
        })?;
        if !score.is_finite() || !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "champion score line {} is outside [0,1]",
                line_number + 1
            ));
        }
        rows.push(ChampionScore {
            id: fields[0].to_owned(),
            score,
        });
    }
    if rows.is_empty() {
        return Err("champion score set is empty".to_owned());
    }
    Ok(rows)
}

fn champion_score(scores: &[ChampionScore], id: &str) -> Result<f32, String> {
    for row in scores {
        if row.id == id {
            return Ok(row.score);
        }
    }
    Err(format!(
        "no independent champion score for traffic row {id}"
    ))
}

fn score(params: ScoringParams, question: &str, truth: &str, answer: &str) -> f32 {
    let raw = scorer::raw_score(question.as_bytes(), truth.as_bytes(), answer.as_bytes());
    public_from_raw(raw, params)
}

fn public_from_raw(raw: f32, params: ScoringParams) -> f32 {
    scorer::public_score_from_raw(raw, params)
}

fn build_raw_corpus(
    fixtures: &[Fixture],
    baseline_fixtures: &[BaselineFixtureScore],
    traffic: &[TrafficRow],
    champion_scores: &[ChampionScore],
) -> Result<RawCorpus, String> {
    if baseline_fixtures.len() != fixtures.len() {
        return Err(format!(
            "baseline fixture score count {} does not match fixture count {}",
            baseline_fixtures.len(),
            fixtures.len()
        ));
    }
    let mut fixture_scores = Vec::with_capacity(fixtures.len());
    for (index, fixture) in fixtures.iter().enumerate() {
        let baseline = &baseline_fixtures[index];
        fixture_scores.push(RawFixtureScores {
            baseline_self_match: baseline.self_match,
            baseline_good: baseline.good,
            baseline_bad: baseline.bad,
            candidate_self_match: cached_raw_score(
                &fixture.question,
                &fixture.ground_truth,
                &fixture.ground_truth,
            ),
            candidate_good: cached_raw_score(
                &fixture.question,
                &fixture.ground_truth,
                &fixture.good,
            ),
            candidate_bad: cached_raw_score(&fixture.question, &fixture.ground_truth, &fixture.bad),
        });
    }
    let mut traffic_scores = Vec::with_capacity(traffic.len());
    for row in traffic {
        traffic_scores.push(RawTrafficScores {
            champion: champion_score(champion_scores, &row.id)?,
            candidate: cached_raw_score(&row.question, &row.ground_truth, &row.answer),
        });
    }

    Ok(RawCorpus {
        fixtures: fixture_scores,
        traffic: traffic_scores,
    })
}

fn write_raw_cache(path: &str, corpus: &RawCorpus) -> Result<(), String> {
    let mut contents = String::from("# isobar-scorer-raw-v4\n");
    for fixture in &corpus.fixtures {
        writeln!(
            &mut contents,
            "F\t{}\t{}\t{}\t{}\t{}\t{}",
            fixture.baseline_self_match.to_bits(),
            fixture.baseline_good.to_bits(),
            fixture.baseline_bad.to_bits(),
            fixture.candidate_self_match.to_bits(),
            fixture.candidate_good.to_bits(),
            fixture.candidate_bad.to_bits()
        )
        .map_err(|error| format!("format raw fixture cache: {error}"))?;
    }
    for traffic in &corpus.traffic {
        writeln!(
            &mut contents,
            "T\t{}\t{}",
            traffic.champion.to_bits(),
            traffic.candidate.to_bits()
        )
        .map_err(|error| format!("format raw traffic cache: {error}"))?;
    }
    fs::write(path, contents).map_err(|error| format!("write raw cache: {error}"))
}

fn read_raw_cache(
    path: &str,
    fixture_count: usize,
    traffic_count: usize,
) -> Result<RawCorpus, String> {
    let contents = fs::read_to_string(path).map_err(|error| format!("read raw cache: {error}"))?;
    if !contents
        .lines()
        .any(|line| line == "# isobar-scorer-raw-v4")
    {
        return Err(
            "raw cache is not an independent baseline cache (expected isobar-scorer-raw-v4)"
                .to_owned(),
        );
    }
    let mut fixtures = Vec::new();
    let mut traffic = Vec::new();
    for (line_number, line) in contents.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            ["F", baseline_self_match, baseline_good, baseline_bad, candidate_self_match, candidate_good, candidate_bad] => {
                fixtures.push(RawFixtureScores {
                    baseline_self_match: f32::from_bits(baseline_self_match.parse().map_err(
                        |error| {
                            format!(
                                "raw cache line {} baseline self-match: {error}",
                                line_number + 1
                            )
                        },
                    )?),
                    baseline_good: f32::from_bits(baseline_good.parse().map_err(|error| {
                        format!(
                            "raw cache line {} baseline good score: {error}",
                            line_number + 1
                        )
                    })?),
                    baseline_bad: f32::from_bits(baseline_bad.parse().map_err(|error| {
                        format!(
                            "raw cache line {} baseline bad score: {error}",
                            line_number + 1
                        )
                    })?),
                    candidate_self_match: f32::from_bits(candidate_self_match.parse().map_err(
                        |error| {
                            format!(
                                "raw cache line {} candidate self-match: {error}",
                                line_number + 1
                            )
                        },
                    )?),
                    candidate_good: f32::from_bits(candidate_good.parse().map_err(|error| {
                        format!(
                            "raw cache line {} candidate good score: {error}",
                            line_number + 1
                        )
                    })?),
                    candidate_bad: f32::from_bits(candidate_bad.parse().map_err(|error| {
                        format!(
                            "raw cache line {} candidate bad score: {error}",
                            line_number + 1
                        )
                    })?),
                })
            }
            ["T", champion, candidate] => traffic.push(RawTrafficScores {
                champion: f32::from_bits(champion.parse().map_err(|error| {
                    format!("raw cache line {} champion score: {error}", line_number + 1)
                })?),
                candidate: f32::from_bits(candidate.parse().map_err(|error| {
                    format!(
                        "raw cache line {} candidate score: {error}",
                        line_number + 1
                    )
                })?),
            }),
            _ => {
                return Err(format!(
                    "raw cache line {} has invalid fields",
                    line_number + 1
                ))
            }
        }
    }
    if fixtures.len() != fixture_count || traffic.len() != traffic_count {
        return Err(format!(
            "raw cache corpus size mismatch: fixtures {} of {}, traffic {} of {}",
            fixtures.len(),
            fixture_count,
            traffic.len(),
            traffic_count
        ));
    }
    Ok(RawCorpus { fixtures, traffic })
}

fn rank_values(values: &[f32]) -> Vec<f64> {
    let mut indices: Vec<usize> = (0..values.len()).collect();
    indices.sort_by(|left, right| {
        values[*left]
            .partial_cmp(&values[*right])
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.cmp(right))
    });

    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < indices.len() {
        let mut end = start + 1;
        while end < indices.len() && values[indices[end]] == values[indices[start]] {
            end += 1;
        }
        let rank = (start + 1 + end) as f64 * 0.5;
        let mut index = start;
        while index < end {
            ranks[indices[index]] = rank;
            index += 1;
        }
        start = end;
    }
    ranks
}

fn pearson(left: &[f64], right: &[f64]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut left_sum = 0.0;
    let mut right_sum = 0.0;
    let mut index = 0;
    while index < left.len() {
        left_sum += left[index];
        right_sum += right[index];
        index += 1;
    }
    let left_mean = left_sum / left.len() as f64;
    let right_mean = right_sum / right.len() as f64;
    let mut numerator = 0.0;
    let mut left_var = 0.0;
    let mut right_var = 0.0;
    index = 0;
    while index < left.len() {
        let left_delta = left[index] - left_mean;
        let right_delta = right[index] - right_mean;
        numerator += left_delta * right_delta;
        left_var += left_delta * left_delta;
        right_var += right_delta * right_delta;
        index += 1;
    }
    if left_var == 0.0 || right_var == 0.0 {
        if left == right {
            1.0
        } else {
            0.0
        }
    } else {
        numerator / (left_var.sqrt() * right_var.sqrt())
    }
}

fn spearman(champion: &[f32], candidate: &[f32]) -> f64 {
    pearson(&rank_values(champion), &rank_values(candidate))
}

fn same_key(left: ScoreKey<'_>, right: ScoreKey<'_>) -> bool {
    left.question == right.question
        && left.ground_truth == right.ground_truth
        && left.answer == right.answer
}

fn tie_counts(values: &[(f32, ScoreKey<'_>)]) -> (usize, usize) {
    let mut distinct_ties = 0;
    let mut duplicate_ties = 0;
    let mut left = 0;
    while left < values.len() {
        let mut right = left + 1;
        while right < values.len() {
            if values[left].0 == values[right].0 {
                if same_key(values[left].1, values[right].1) {
                    duplicate_ties += 1;
                } else {
                    distinct_ties += 1;
                }
            }
            right += 1;
        }
        left += 1;
    }
    (distinct_ties, duplicate_ties)
}

fn percentile(sorted: &[u128], numerator: usize, denominator: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let last = sorted.len() - 1;
    let index = (last * numerator + denominator / 2) / denominator;
    sorted[index.min(last)]
}

fn measure_latency(params: ScoringParams, fixtures: &[Fixture], sample_count: usize) -> Latency {
    let mut samples = Vec::with_capacity(sample_count);
    for index in 0..sample_count {
        let fixture = &fixtures[index % fixtures.len()];
        let started = Instant::now();
        let value = score(
            params,
            &fixture.question,
            &fixture.ground_truth,
            &fixture.good,
        );
        black_box(value);
        samples.push(started.elapsed().as_nanos());
    }
    samples.sort_unstable();
    Latency {
        p50_ns: percentile(&samples, 50, 100),
        p99_ns: percentile(&samples, 99, 100),
    }
}

fn determinism_check(params: ScoringParams, fixtures: &[Fixture], iteration_count: usize) -> bool {
    if iteration_count == 0 {
        return true;
    }
    let fixture = &fixtures[0];
    let reference_raw = scorer::raw_score(
        fixture.question.as_bytes(),
        fixture.ground_truth.as_bytes(),
        fixture.good.as_bytes(),
    );
    let reference_score = public_from_raw(reference_raw, params);
    for _ in 0..iteration_count {
        let current_raw = scorer::raw_score(
            fixture.question.as_bytes(),
            fixture.ground_truth.as_bytes(),
            fixture.good.as_bytes(),
        );
        let current_score = public_from_raw(current_raw, params);
        if reference_raw.to_bits() != current_raw.to_bits()
            || reference_score.to_bits() != current_score.to_bits()
        {
            return false;
        }
    }
    true
}

fn evaluate(
    params: ScoringParams,
    fixtures: &[Fixture],
    traffic: &[TrafficRow],
    raw_corpus: &RawCorpus,
    determinism_iterations: usize,
    latency_samples: usize,
) -> Metrics {
    let mut self_match = 1.0f32;
    let mut margin_sum = 0.0f64;
    let mut ordering = 0;
    let mut candidate_scores = Vec::with_capacity(raw_corpus.fixtures.len() * 2);
    for (index, raw) in raw_corpus.fixtures.iter().enumerate() {
        let fixture = &fixtures[index];
        let self_score = public_from_raw(raw.candidate_self_match, params);
        let good = public_from_raw(raw.candidate_good, params);
        let bad = public_from_raw(raw.candidate_bad, params);
        self_match = self_match.min(self_score);
        margin_sum += (good - bad) as f64;
        if good > bad {
            ordering += 1;
        }
        candidate_scores.push((
            good,
            ScoreKey {
                question: &fixture.question,
                ground_truth: &fixture.ground_truth,
                answer: &fixture.good,
            },
        ));
        candidate_scores.push((
            bad,
            ScoreKey {
                question: &fixture.question,
                ground_truth: &fixture.ground_truth,
                answer: &fixture.bad,
            },
        ));
    }

    let mut champion_scores = Vec::with_capacity(raw_corpus.traffic.len());
    let mut candidate_traffic_scores = Vec::with_capacity(raw_corpus.traffic.len());
    let mut traffic_values = Vec::with_capacity(raw_corpus.traffic.len());
    for (index, raw) in raw_corpus.traffic.iter().enumerate() {
        let row = &traffic[index];
        champion_scores.push(raw.champion);
        let score = public_from_raw(raw.candidate, params);
        candidate_traffic_scores.push(score);
        traffic_values.push((
            score,
            ScoreKey {
                question: &row.question,
                ground_truth: &row.ground_truth,
                answer: &row.answer,
            },
        ));
    }
    let agreement = spearman(&champion_scores, &candidate_traffic_scores);
    let (fixture_ties, fixture_duplicate_ties) = tie_counts(&candidate_scores);
    let (traffic_ties, traffic_duplicate_ties) = tie_counts(&traffic_values);
    Metrics {
        self_match,
        margin: margin_sum / fixtures.len() as f64,
        ordering,
        fixture_total: fixtures.len(),
        agreement,
        ties: fixture_ties + traffic_ties,
        duplicate_ties: fixture_duplicate_ties + traffic_duplicate_ties,
        deterministic: determinism_check(params, fixtures, determinism_iterations),
        determinism_iterations,
        latency: measure_latency(params, fixtures, latency_samples),
        latency_samples,
    }
}

fn baseline_margin(fixtures: &[RawFixtureScores]) -> (f64, usize) {
    let mut total = 0.0f64;
    let mut ordering = 0;
    for fixture in fixtures {
        let good = fixture.baseline_good;
        let bad = fixture.baseline_bad;
        total += (good - bad) as f64;
        if good > bad {
            ordering += 1;
        }
    }
    (total / fixtures.len() as f64, ordering)
}

fn print_report(metrics: &Metrics, champion_margin: f64, champion_ordering: usize) -> bool {
    let margin_pass = metrics.margin > champion_margin;
    let ordering_pass = metrics.ordering >= champion_ordering;
    let self_pass = metrics.self_match >= MIN_SELF_MATCH;
    let floor_pass = metrics.margin >= MIN_MARGIN;
    let agreement_pass = metrics.agreement >= MIN_AGREEMENT;
    println!(
        "self-match       {:.6}  [{}]",
        metrics.self_match,
        if self_pass { "pass" } else { "FAIL" }
    );
    println!(
        "average margin    {:.6}  [{}]",
        metrics.margin,
        if floor_pass { "pass" } else { "FAIL" }
    );
    println!(
        "ordering          {} of {}  [{}]",
        metrics.ordering,
        metrics.fixture_total,
        if ordering_pass { "pass" } else { "FAIL" }
    );
    println!(
        "rank agreement    {:.6}  [{}]",
        metrics.agreement,
        if agreement_pass { "pass" } else { "FAIL" }
    );
    println!("ties (distinct)    {}", metrics.ties);
    println!("duplicate-input ties {}", metrics.duplicate_ties);
    println!(
        "determinism       {} iterations [{}]",
        metrics.determinism_iterations,
        if metrics.deterministic {
            "pass"
        } else {
            "FAIL"
        }
    );
    println!(
        "latency           {} samples p50={} ns p99={} ns",
        metrics.latency_samples, metrics.latency.p50_ns, metrics.latency.p99_ns
    );
    println!(
        "baseline reference margin {:.6}, ordering {} of {}",
        champion_margin, champion_ordering, metrics.fixture_total
    );
    println!(
        "strict separation [{}]",
        if margin_pass { "pass" } else { "FAIL" }
    );
    self_pass
        && floor_pass
        && ordering_pass
        && agreement_pass
        && margin_pass
        && metrics.deterministic
}

fn usage() {
    eprintln!("usage: isobar-scorer-harness [--fixtures PATH] [--baseline-fixtures PATH] [--traffic PATH] [--corpus PATH] [--champion-scores PATH] [--raw-cache PATH] [--k VALUE] [--c VALUE] [--champion-margin VALUE] [--champion-ordering VALUE] [--determinism-iterations N] [--latency-samples N] [--sweep] [--sweep-k-max VALUE] [--sweep-centre-max VALUE]");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut fixture_path = None;
    let mut baseline_fixture_path = None;
    let mut traffic_path = None;
    let mut corpus_path = None;
    let mut champion_scores_path = None;
    let mut raw_cache_path = None;
    let mut steepness = scorer::DEFAULT_STEEPNESS;
    let mut centre = scorer::DEFAULT_CENTRE;
    let mut champion_margin_override = None;
    let mut champion_ordering_override = None;
    let mut determinism_iterations = DEFAULT_DETERMINISM_ITERATIONS;
    let mut latency_samples = DEFAULT_LATENCY_SAMPLES;
    let mut sweep_k_max = SWEEP_K_MAX;
    let mut sweep_c_max = SWEEP_C_MAX;
    let mut sweep = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--fixtures" => {
                index += 1;
                fixture_path = args.get(index).cloned();
            }
            "--baseline-fixtures" => {
                index += 1;
                baseline_fixture_path = args.get(index).cloned();
            }
            "--traffic" => {
                index += 1;
                traffic_path = args.get(index).cloned();
            }
            "--corpus" => {
                index += 1;
                corpus_path = args.get(index).cloned();
            }
            "--champion-scores" => {
                index += 1;
                champion_scores_path = args.get(index).cloned();
            }
            "--raw-cache" => {
                index += 1;
                raw_cache_path = args.get(index).cloned();
            }
            "--k" => {
                index += 1;
                steepness = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(scorer::DEFAULT_STEEPNESS);
            }
            "--c" => {
                index += 1;
                centre = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(scorer::DEFAULT_CENTRE);
            }
            "--champion-margin" => {
                index += 1;
                champion_margin_override = args.get(index).and_then(|value| value.parse().ok());
            }
            "--champion-ordering" => {
                index += 1;
                champion_ordering_override = args.get(index).and_then(|value| value.parse().ok());
            }
            "--determinism-iterations" => {
                index += 1;
                determinism_iterations = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_DETERMINISM_ITERATIONS);
            }
            "--latency-samples" => {
                index += 1;
                latency_samples = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_LATENCY_SAMPLES);
            }
            "--sweep-k-max" => {
                index += 1;
                sweep_k_max = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(SWEEP_K_MAX);
            }
            "--sweep-centre-max" => {
                index += 1;
                sweep_c_max = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(SWEEP_C_MAX);
            }
            "--sweep" => sweep = true,
            "--help" | "-h" => {
                usage();
                return;
            }
            unknown => {
                eprintln!("unknown option: {unknown}");
                usage();
                std::process::exit(2);
            }
        }
        index += 1;
    }

    let fixture_contents = fixture_path
        .as_deref()
        .map(fs::read_to_string)
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("could not read fixtures: {error}");
            std::process::exit(2);
        })
        .unwrap_or_else(|| DEFAULT_FIXTURES.to_owned());
    let traffic_source = corpus_path.as_deref().or(traffic_path.as_deref());
    let traffic_contents = traffic_source
        .map(fs::read_to_string)
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("could not read traffic: {error}");
            std::process::exit(2);
        })
        .unwrap_or_else(|| DEFAULT_TRAFFIC.to_owned());
    let baseline_fixture_contents = baseline_fixture_path
        .as_deref()
        .map(fs::read_to_string)
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("could not read baseline fixtures: {error}");
            std::process::exit(2);
        })
        .unwrap_or_else(|| DEFAULT_BASELINE_FIXTURES.to_owned());
    let champion_scores_contents = champion_scores_path
        .as_deref()
        .map(fs::read_to_string)
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("could not read champion scores: {error}");
            std::process::exit(2);
        })
        .unwrap_or_else(|| DEFAULT_CHAMPION_SCORES.to_owned());
    let parsed_traffic = parse_traffic(&traffic_contents).unwrap_or_else(|error| {
        eprintln!("could not parse traffic: {error}");
        std::process::exit(2);
    });
    let (fixtures, baseline_fixtures, traffic) = if corpus_path.is_some() {
        let has_intent_prefix = parsed_traffic
            .iter()
            .any(|row| row.id.starts_with("weather-check-"));
        let rows = if has_intent_prefix {
            parsed_traffic
                .into_iter()
                .filter(|row| row.id.starts_with("weather-check-"))
                .collect::<Vec<_>>()
        } else {
            parsed_traffic
        };
        if rows.is_empty() {
            eprintln!("corpus contains no WEATHER_CHECK rows");
            std::process::exit(2);
        }
        let (fixtures, baseline) = corpus_fixtures(&rows);
        (fixtures, baseline, rows)
    } else {
        let fixtures = parse_fixtures(&fixture_contents).unwrap_or_else(|error| {
            eprintln!("could not parse fixtures: {error}");
            std::process::exit(2);
        });
        let baseline = if fixture_path.is_some() && baseline_fixture_path.is_none() {
            // A generated real-traffic fixture set has no independent baseline
            // vector. Keep the comparison fields neutral while still measuring
            // the candidate's self-match, margin, ordering, and traffic rank.
            fixtures
                .iter()
                .map(|_| BaselineFixtureScore {
                    self_match: 1.0,
                    good: 1.0,
                    bad: 0.0,
                })
                .collect()
        } else {
            parse_baseline_fixtures(&baseline_fixture_contents).unwrap_or_else(|error| {
                eprintln!("could not parse baseline fixtures: {error}");
                std::process::exit(2);
            })
        };
        (fixtures, baseline, parsed_traffic)
    };
    let mut champion_scores =
        parse_champion_scores(&champion_scores_contents).unwrap_or_else(|error| {
            eprintln!("could not parse champion scores: {error}");
            std::process::exit(2);
        });
    if corpus_path.is_some()
        && champion_scores
            .iter()
            .any(|row| row.id.starts_with("weather-check-"))
    {
        champion_scores.retain(|row| row.id.starts_with("weather-check-"));
    }

    let raw_corpus = match raw_cache_path.as_deref() {
        Some(path) if Path::new(path).exists() => {
            read_raw_cache(path, fixtures.len(), traffic.len()).unwrap_or_else(|error| {
                eprintln!("could not read raw cache: {error}");
                std::process::exit(2);
            })
        }
        Some(path) => {
            let corpus =
                build_raw_corpus(&fixtures, &baseline_fixtures, &traffic, &champion_scores)
                    .unwrap_or_else(|error| {
                        eprintln!("could not build raw corpus: {error}");
                        std::process::exit(2);
                    });
            write_raw_cache(path, &corpus).unwrap_or_else(|error| {
                eprintln!("could not write raw cache: {error}");
                std::process::exit(2);
            });
            corpus
        }
        None => build_raw_corpus(&fixtures, &baseline_fixtures, &traffic, &champion_scores)
            .unwrap_or_else(|error| {
                eprintln!("could not build raw corpus: {error}");
                std::process::exit(2);
            }),
    };
    let (proxy_margin, proxy_ordering) = baseline_margin(&raw_corpus.fixtures);
    let champion_margin = champion_margin_override.unwrap_or(proxy_margin);
    let champion_ordering = champion_ordering_override.unwrap_or(proxy_ordering);
    if sweep {
        println!("k,centre,margin,agreement,self_match,ordering,total,ties");
        let mut k = SWEEP_K_START;
        while k <= sweep_k_max && k.is_finite() {
            let mut c = SWEEP_C_START;
            while c <= sweep_c_max && c.is_finite() {
                let metrics = evaluate(
                    ScoringParams {
                        steepness: k,
                        centre: c,
                        threshold: false,
                    },
                    &fixtures,
                    &traffic,
                    &raw_corpus,
                    0,
                    0,
                );
                println!(
                    "{k:.1},{c:.1},{:.6},{:.6},{:.6},{},{},{}",
                    metrics.margin,
                    metrics.agreement,
                    metrics.self_match,
                    metrics.ordering,
                    metrics.fixture_total,
                    metrics.ties
                );
                c += SWEEP_C_STEP;
            }
            k += SWEEP_K_STEP;
        }
        return;
    }

    let metrics = evaluate(
        ScoringParams {
            steepness,
            centre,
            threshold: true,
        },
        &fixtures,
        &traffic,
        &raw_corpus,
        determinism_iterations,
        latency_samples,
    );
    let pass = print_report(&metrics, champion_margin, champion_ordering);
    if !pass {
        std::process::exit(1);
    }
}
