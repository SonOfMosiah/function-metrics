# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-07-12

### Added

- Select duration histograms, call counters, and returned-error counters per function.
- Classify domain-specific return values with `error_classifier`.
- Register stable metric descriptions and seconds units for duration histograms.

### Changed

- Instrument native async functions without adding a measured-future wrapper.
- Reserve Prometheus-generated label names and metric suffixes at compile time.

## [0.1.0] - 2026-07-10

### Added

- Add `#[function_metrics]` duration instrumentation.
- Support static, dynamic, expression, and named-field labels.
- Support sync, native async, and `async-trait` functions.
- Validate operation and label names at compile time.
- Record durations when instrumented functions panic or polled futures are cancelled.
- Reject `#[track_caller]` and non-async functions returning `impl Future` with targeted diagnostics.

[Unreleased]: https://github.com/SonOfMosiah/function-metrics/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/SonOfMosiah/function-metrics/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/SonOfMosiah/function-metrics/releases/tag/v0.1.0
