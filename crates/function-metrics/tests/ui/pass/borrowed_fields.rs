use function_metrics::function_metrics;

struct Request {
    status: String,
    region: String,
}

#[function_metrics(name = "borrowed_fields", labels(status = request.status, request.region))]
fn process(request: &Request) -> usize {
    request.status.len() + request.region.len()
}

fn main() {
    let request = Request {
        status: "ok".to_owned(),
        region: "us".to_owned(),
    };
    assert_eq!(process(&request), 4);
}
