use function_metrics::function_metrics;

#[function_metrics(metrics(errors))]
fn unsupported() -> u16 { 500 }

fn main() {}
