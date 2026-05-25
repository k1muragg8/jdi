#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

echo "==> Reset runtime-mutated example state to avoid git pull conflict"

if git status --short examples/portfolio_state.json | grep -q .; then
  echo "examples/portfolio_state.json has local changes. Restoring it..."
  git restore examples/portfolio_state.json
fi

echo "==> Pull latest code"
git pull --ff-only

echo "==> Format"
cargo fmt

echo "==> Check"
cargo check

echo "==> Test"
cargo test

echo "==> Mark to market"
cargo run -- mtm

echo "==> Show holdings"
cargo run -- holdings

echo "==> Done"
