use function_metrics::function_metrics;

#[function_metrics(metrics(duration, duration))]
fn duplicate() {}

fn main() {}
