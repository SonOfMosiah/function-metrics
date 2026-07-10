use function_metrics::function_metrics;

#[function_metrics(labels(chain_id, chain_id))]
fn duplicate(chain_id: u64) {}

fn main() {}
