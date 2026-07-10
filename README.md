# function-metrics

Prometheus-native function instrumentation for Rust's
[`metrics`](https://crates.io/crates/metrics) facade.

`function-metrics` turns a small attribute into a consistently named metric
family while preserving useful domain labels such as `chain_id`, `provider`,
or `dex`.

> The first release implements duration histograms. Call counts, error counts,
> and in-flight gauges are planned behind the same operation-name interface.

## Usage

```toml
[dependencies]
function-metrics = "0.1"
```

Until the first crates.io release, use the Git dependency instead:

```toml
function-metrics = { git = "https://github.com/SonOfMosiah/function-metrics" }
```

```rust
use function_metrics::function_metrics;

#[function_metrics(
    name = "quote_evm",
    labels(chain_id, dex = "uniswap_v3")
)]
async fn quote(chain_id: ChainId) -> Result<Quote, QuoteError> {
    // ...
}
```

This records fractional seconds to:

```text
quote_evm_duration_seconds{chain_id="8453",dex="uniswap_v3"}
```

The operation name defaults to the Rust function name:

```rust
#[function_metrics]
async fn refresh_cache() {
    // Emits refresh_cache_duration_seconds
}
```

The application remains responsible for installing a `metrics` recorder, such
as [`metrics-exporter-prometheus`](https://crates.io/crates/metrics-exporter-prometheus).

## Labels

Labels may be static strings, function parameters, expressions with an
explicit key, or named fields:

```rust
#[function_metrics(
    name = "request",
    labels(
        chain_id,
        provider = "primary",
        status = request.status,
        request.region,
    )
)]
async fn request(chain_id: ChainId, request: Request) {
    // ...
}
```

Dynamic values must implement `ToString`. Metric and label names are validated
as snake_case at compile time, and duplicate label keys are rejected.

Use only bounded label dimensions. Network identifiers, providers, protocols,
and finite outcome categories are usually appropriate; user IDs, transaction
hashes, addresses, and arbitrary error messages are not.

## Execution semantics

- Sync, native async, and `async-trait` functions are supported.
- The full future execution is timed, not merely future construction.
- Normal returns, including `return` and `?`, record a duration.
- Panics and cancelled/dropped futures do not currently record a duration.
- Label values are captured before the timer starts and before the function
  body can consume its arguments.

## Metric naming

An operation named `quote_evm` emits `quote_evm_duration_seconds`. Durations
use Prometheus's base time unit and are recorded through
`Histogram::record(Duration)`, preserving sub-millisecond precision.

Future metric types will share the same base name:

```text
quote_evm_calls_total
quote_evm_errors_total
quote_evm_duration_seconds
quote_evm_in_flight
```

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

For a release, publish `function-metrics-macros` first. After that version is
visible in the crates.io index, package and publish `function-metrics`; Cargo
requires every non-development dependency to already exist in the registry.

## License

GPL-3.0-only. This implementation was extracted from Zenithar, whose source is
distributed under GPL-3.0.
