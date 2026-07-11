# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Record durations when instrumented functions panic or polled futures are cancelled.
- Reject `#[track_caller]` and non-async functions returning `impl Future` with targeted diagnostics.

## [0.1.0] - 2026-07-10

### Added

- Add `#[function_metrics]` duration instrumentation.
- Support static, dynamic, expression, and named-field labels.
- Support sync, native async, and `async-trait` functions.
- Validate operation and label names at compile time.

[Unreleased]: https://github.com/SonOfMosiah/function-metrics/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/SonOfMosiah/function-metrics/releases/tag/v0.1.0
