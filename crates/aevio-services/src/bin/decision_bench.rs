use std::time::{Duration, Instant};

use aevio_services::decision::use_cases;
use aevio_services::decision::{
    DecisionProvider, DecisionResult, RemoteProtocol, RemoteProvider, RuleProvider,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    provider: String,
    iterations: usize,
    successes: usize,
    abstentions: usize,
    errors: usize,
    p50_us: u128,
    p95_us: u128,
    p99_us: u128,
    accuracy: Option<f64>,
    calibration_error: Option<f64>,
    token_consumption: Option<u64>,
    provider_cost: Option<f64>,
}

fn request() -> aevio_services::decision::DecisionRequest {
    use_cases::task_prioritization(
        vec!["task-a".into(), "task-b".into(), "task-c".into()],
        Some("task-b".into()),
    )
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    if samples.is_empty() {
        return 0;
    }
    let index = ((samples.len() - 1) * percentile).div_ceil(100);
    samples[index]
}

fn report(
    provider: String,
    durations: &mut [u128],
    results: &[Result<DecisionResult, String>],
) -> BenchmarkReport {
    durations.sort_unstable();
    let successes = results.iter().filter(|result| result.is_ok()).count();
    let abstentions = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .filter(|result| result.abstention.is_some())
        .count();
    BenchmarkReport {
        provider,
        iterations: results.len(),
        successes,
        abstentions,
        errors: results.len().saturating_sub(successes),
        p50_us: percentile(durations, 50),
        p95_us: percentile(durations, 95),
        p99_us: percentile(durations, 99),
        // Accuracy and calibration require labeled evaluation data; do not invent values.
        accuracy: None,
        calibration_error: None,
        token_consumption: None,
        provider_cost: None,
    }
}

fn run_rules(iterations: usize) -> BenchmarkReport {
    let provider = RuleProvider;
    let mut durations = Vec::with_capacity(iterations);
    let mut results = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let result = futures_lite::future::block_on(provider.decide(&request(), "benchmark"))
            .map_err(|error| error.to_string());
        durations.push(started.elapsed().as_micros());
        results.push(result);
    }
    report("rules".into(), &mut durations, &results)
}

fn run_remote(
    iterations: usize,
    protocol: RemoteProtocol,
    endpoint: String,
    model: String,
    key: Option<String>,
) -> BenchmarkReport {
    let name = match protocol {
        RemoteProtocol::Laya => "laya",
        RemoteProtocol::Jev => "jev",
    };
    let provider = match RemoteProvider::new(endpoint, model, key, protocol, Duration::from_secs(5))
    {
        Ok(provider) => provider,
        Err(_) => {
            return BenchmarkReport {
                provider: name.into(),
                iterations,
                successes: 0,
                abstentions: 0,
                errors: iterations,
                p50_us: 0,
                p95_us: 0,
                p99_us: 0,
                accuracy: None,
                calibration_error: None,
                token_consumption: None,
                provider_cost: None,
            };
        }
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("benchmark runtime must be available");
    let mut durations = Vec::with_capacity(iterations);
    let mut results = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        let result = runtime
            .block_on(provider.decide(&request(), "benchmark"))
            .map_err(|error| error.to_string());
        durations.push(started.elapsed().as_micros());
        results.push(result);
    }
    report(name.into(), &mut durations, &results)
}

fn iterations() -> usize {
    std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .find(|pair| pair[0] == "--iterations")
        .and_then(|pair| pair[1].parse().ok())
        .filter(|value: &usize| (1..=10_000).contains(value))
        .unwrap_or(100)
}

fn selected_provider() -> String {
    std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .find(|pair| pair[0] == "--provider")
        .map_or_else(|| "rules".into(), |pair| pair[1].clone())
}

fn main() {
    let count = iterations();
    let selected = selected_provider();
    let mut reports = vec![run_rules(count)];
    if (selected == "laya" || selected == "all")
        && let Ok(endpoint) = std::env::var("AEVIO_LAYA_BENCH_ENDPOINT")
    {
        reports.push(run_remote(
            count,
            RemoteProtocol::Laya,
            endpoint,
            std::env::var("AEVIO_LAYA_BENCH_MODEL").unwrap_or_else(|_| "typed-decisions".into()),
            None,
        ));
    }
    if (selected == "jev" || selected == "all")
        && let Ok(key) = std::env::var("AEVIO_JEV_API_KEY")
    {
        reports.push(run_remote(
            count,
            RemoteProtocol::Jev,
            std::env::var("AEVIO_JEV_BENCH_ENDPOINT")
                .unwrap_or_else(|_| "https://thejevai.com/v1/systemone".into()),
            std::env::var("AEVIO_JEV_BENCH_MODEL").unwrap_or_else(|_| "typesafe/jev-1.14".into()),
            Some(key),
        ));
    }
    match serde_json::to_string_pretty(&reports) {
        Ok(output) => println!("{output}"),
        Err(error) => eprintln!("could not serialize benchmark report: {error}"),
    }
}
