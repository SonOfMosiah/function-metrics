use function_metrics::function_metrics;

#[function_metrics(labels(le = "1.0"))]
fn reserved() {}

fn main() {}
