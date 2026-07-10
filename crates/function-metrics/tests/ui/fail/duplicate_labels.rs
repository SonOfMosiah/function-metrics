use function_metrics::function_metrics;

#[function_metrics(labels(method, method))]
fn duplicate(method: &str) {}

fn main() {}
