use function_metrics::function_metrics;

fn is_error(_: &u16) -> bool { false }

#[function_metrics(metrics(calls), error_classifier = is_error)]
fn classified() -> u16 { 200 }

fn main() {}
