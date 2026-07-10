use fm::function_metrics;

#[function_metrics(name = "renamed_dependency")]
fn instrumented() {}

fn main() {
    instrumented();
}
