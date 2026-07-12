use function_metrics::function_metrics;

#[function_metrics(metrics(histogram))]
fn unknown() {}

fn main() {}
