use std::pin::Pin;

use function_metrics::function_metrics;

mod custom {
    pub trait Future {}
}

struct Value;

impl custom::Future for Value {}

#[function_metrics]
fn construct_future() -> Pin<Box<dyn custom::Future>> {
    Box::pin(Value)
}

fn main() {
    let _ = construct_future();
}
