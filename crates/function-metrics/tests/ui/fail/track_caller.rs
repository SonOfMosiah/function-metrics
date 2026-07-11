use function_metrics::function_metrics;

#[function_metrics]
#[track_caller]
fn caller_location() -> &'static std::panic::Location<'static> {
    std::panic::Location::caller()
}

fn main() {}
