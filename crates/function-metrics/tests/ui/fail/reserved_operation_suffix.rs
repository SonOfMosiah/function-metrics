use function_metrics::function_metrics;

#[function_metrics(name = "already_calls_total", metrics(calls))]
fn reserved() {}

fn main() {}
