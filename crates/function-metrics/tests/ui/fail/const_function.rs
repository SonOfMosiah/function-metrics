use function_metrics::function_metrics;

#[function_metrics]
const fn constant() -> u64 {
    1
}

fn main() {}
