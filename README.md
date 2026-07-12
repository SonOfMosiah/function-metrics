# function-metrics

Prometheus-native function instrumentation for Rust's
[`metrics`](https://crates.io/crates/metrics) facade.

`function-metrics` turns an attribute into a consistently named metric family
while preserving bounded application labels such as `method`, `service`, or
`status`.

## Usage

```toml
[dependencies]
function-metrics = "0.2"
```

Duration-only instrumentation remains the default:

```rust
use function_metrics::function_metrics;

#[function_metrics(name = "handle_request", labels(method, service = "api"))]
async fn handle_request(method: Method) -> Result<Response, RequestError> {
    // ...
}
```

This records fractional seconds to:

```text
handle_request_duration_seconds{method="GET",service="api"}
```

Select additional instruments with `metrics(...)`:

```rust
#[function_metrics(
    name = "handle_request",
    metrics(duration, calls, errors),
    labels(method, service = "api"),
)]
async fn handle_request(method: Method) -> Result<Response, RequestError> {
    // ...
}
```

| Selection | Emitted family | Prometheus type |
|---|---|---|
| `duration` | `handle_request_duration_seconds` | histogram |
| `calls` | `handle_request_calls_total` | counter |
| `errors` | `handle_request_errors_total` | counter |

Omitting `metrics(...)` is equivalent to `metrics(duration)`. The operation
name defaults to the Rust function name when `name` is omitted.

The application remains responsible for installing a `metrics` recorder, such
as [`metrics-exporter-prometheus`](https://crates.io/crates/metrics-exporter-prometheus).
Histogram buckets and classic-versus-native histogram policy belong in that
recorder configuration, not on individual functions.

## Errors

For a syntactically visible `Result<T, E>` return type, `errors` increments
when the function returns `Err`. Panics and cancelled futures terminate a call
but are not application errors.

Type aliases and domain-specific outcomes can use an explicit classifier:

```rust
fn response_is_error(response: &Response) -> bool {
    response.status >= 400
}

#[function_metrics(
    name = "send_request",
    metrics(calls, errors),
    error_classifier = response_is_error,
)]
async fn send_request() -> Response {
    // ...
}
```

A classifier has the signature `fn(&ReturnType) -> bool` and runs once after a
normal return. It must be deterministic and inexpensive.

## Labels

Labels may be static strings, function parameters, expressions with an
explicit key, or named fields:

```rust
#[function_metrics(
    name = "process_request",
    labels(
        method,
        service = "api",
        status = request.status,
        request.region,
    )
)]
async fn process_request(method: Method, request: Request) {
    // ...
}
```

Dynamic values must implement `ToString`. Metric and label names are validated
as snake_case. Duplicate keys and Prometheus-reserved keys such as `le`,
`quantile`, and `__name__` are rejected.

Use only bounded label dimensions. HTTP methods, deployment environments, and
finite outcome categories are usually appropriate. User IDs, request IDs,
file paths, URLs, hashes, addresses, and arbitrary error messages are not.

## Execution semantics

- Sync, native async, and `async-trait` functions are supported.
- Async instrumentation starts on first poll, not future construction.
- Duration and calls record once on normal returns, propagated errors, panics,
  and cancellation after polling starts.
- Dropping a future before its first poll records nothing.
- Errors count returned `Err` values or classifier matches only.
- Labels are captured once before the timer starts and reused by every enabled
  instrument.
- Generated metric descriptions are registered with the recorder; duration is
  described with the `seconds` unit.

Functions marked `#[track_caller]` are rejected because wrapping their bodies
would change `Location::caller()`. Non-async functions returning `impl Future`
are also rejected; use `async fn` so execution can be timed. Future traits
imported under a different name and concrete future return aliases cannot
always be detected by an attribute macro and should not be annotated after
expansion.

## Prometheus and Grafana queries

A duration histogram already includes an observation count. When `duration` is
enabled, call rate can often be derived without also enabling `calls`:

```promql
rate(handle_request_duration_seconds_count[$__rate_interval])
```

Classic histogram p95:

```promql
histogram_quantile(
  0.95,
  sum by (le) (
    rate(handle_request_duration_seconds_bucket[$__rate_interval])
  )
)
```

Native histogram p95:

```promql
histogram_quantile(
  0.95,
  sum(rate(handle_request_duration_seconds[$__rate_interval]))
)
```

Error ratio when calls and errors are enabled:

```promql
sum(rate(handle_request_errors_total[$__rate_interval]))
/
sum(rate(handle_request_calls_total[$__rate_interval]))
```

Use classic bucket boundaries that match important SLO thresholds. Native
histograms can provide broader automatic resolution when the exporter,
Prometheus scrape configuration, and remote-write path all support them.
Grafana's `$__rate_interval` is appropriate for interactive dashboards; use a
fixed range in recording and alerting rules.

The `metrics` 0.24 facade does not expose an exemplar API, so trace IDs cannot
currently be attached as exemplars by this crate. Never put trace IDs into
ordinary metric labels.

## Repository structure

- `function-metrics` is the public facade and owns the `metrics` dependency.
- `function-metrics-macros` parses and expands `#[function_metrics]`.

Consumers should depend only on `function-metrics`. The split prevents macro
expansions from requiring a separately named `metrics` dependency and supports
renaming the facade dependency in `Cargo.toml`.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path tests/renamed-dependency/Cargo.toml
```

The minimum supported Rust version is 1.86.

See [`CONTRIBUTING.md`](https://github.com/SonOfMosiah/function-metrics/blob/main/CONTRIBUTING.md)
for Conventional Commit requirements and automated release/changelog details.

For a release, publish `function-metrics-macros` first. After that version is
visible in the crates.io index, package and publish `function-metrics`; Cargo
requires every non-development dependency to already exist in the registry.

## License

MIT. See [`LICENSE`](LICENSE).
