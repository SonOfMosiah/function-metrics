#!/usr/bin/env bash
set -euo pipefail

# The facade can only be packaged after the matching macro version is visible
# in the crates.io index. Release function-metrics-macros first, then run this.
cargo package -p function-metrics
