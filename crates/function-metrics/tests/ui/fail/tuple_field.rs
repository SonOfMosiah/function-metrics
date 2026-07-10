use function_metrics::function_metrics;

#[function_metrics(labels(value.0))]
fn tuple(value: (u64, u64)) {}

fn main() {}
