use function_metrics::function_metrics;

struct Runner;

impl Runner {
    #[function_metrics(name = "generic_method", labels(kind))]
    fn run<T: ToString>(kind: T) -> T {
        kind
    }
}

fn main() {
    assert_eq!(Runner::run("quote"), "quote");
}
