---
name: coverage
description: >
  Analyze test coverage for the exergon project. Regenerates coverage data and
  reports per-file line/function coverage, highlights uncovered project functions,
  and surfaces gaps worth testing.
  Triggers on: /coverage, "what's covered", "test coverage", "coverage report",
  "what needs tests", "coverage gaps".
user-invocable: true
allowed-tools: Bash(cargo llvm-cov *)
---

# Coverage Analysis

## Step 1: Regenerate coverage data

```bash
cargo llvm-cov --json --output-path coverage.json -q
```

Requires `cargo-llvm-cov`. Install if missing: `cargo install cargo-llvm-cov`.

## Step 2: Analyze with this script

Run the Python script ${CLAUDE_SKILL_DIR}/scripts/read_coverage.py coverage.py

## Step 3: Interpret results
**Reading the output:**
- `lines %` = executable lines hit by at least one test
- `fn %` = functions called by at least one test
- Mangled symbols: `_RNvNtNt...exergon5drone5testss_28...` → test functions are prefixed with `tests`
- Non-test uncovered functions = production code with no test path

**What to prioritize:**
1. Pure logic functions (no ECS / no `World` needed) — easiest wins
2. Systems with clear inputs/outputs — test via `World` directly (see `bevy/ecs.md`)
3. Skip: `main.rs`, asset loaders, UI layout, anything requiring a render context
