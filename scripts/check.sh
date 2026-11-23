#!/usr/bin/env bash
# Quiet checks: scripts/check.sh fast|affected|full [--e2e]
#   fast      fmt, clippy, content validate, generated files up to date
#   affected  fast, plus tests of the crates changed against BASE (and their
#             dependants), plus web typecheck/lint/test if web/ changed
#   full      what CI's rust and web jobs run (Playwright only with --e2e)
# Full logs: target/check-logs/<step>.log. The terminal gets one line per step
# and a summary of at most ten lines. Honours CARGO_TARGET_DIR. BASE (default
# origin/claude/bold-brown-shyeug) is the branch "affected" compares against.
set -uo pipefail
cd "$(dirname "$0")/.."

mode="${1:-fast}"; e2e=0
[[ "${2:-}" == "--e2e" ]] && e2e=1
case "$mode" in fast|affected|full) ;; *) echo "usage: scripts/check.sh fast|affected|full [--e2e]" >&2; exit 2 ;; esac

BASE="${BASE:-origin/claude/bold-brown-shyeug}"
LOGS="target/check-logs"
mkdir -p "$LOGS"
passed=0; failed=();

# step <name> <command...>: run quietly; one line on the terminal.
step() {
  local name="$1"; shift
  local log="$LOGS/$name.log"
  if "$@" >"$log" 2>&1; then
    echo "PASS $name"; passed=$((passed + 1))
  else
    echo "FAIL $name (log: $log)"; failed+=("$name")
    grep -nE '^(error|warning: unused)|^error\[|FAILED|panicked|failed|ERROR|✖|Error:|differ|^diff ' "$log" | head -15
    [[ -s "$log" ]] && ! grep -qE 'error|FAILED|panicked|failed|ERROR|✖|differ|^diff ' "$log" && tail -n 15 "$log"
  fi
}
sh_step() { local name="$1"; shift; step "$name" bash -c "$*"; }

run_fast() {
  step fmt cargo fmt --all --check
  step clippy cargo clippy --workspace --all-targets -- -D warnings
  step content-validate cargo run -q -p mandate-cli -- content validate
  sh_step generated 'cargo run -q -p mandate-cli -- content schema && git diff --exit-code content/schemas web/src/engine/protocol'
}

# Crates in dependency order, with the crates each depends on (static map).
ALL_CRATES=(mandate-core mandate-content mandate-sim mandate-worldgen mandate-protocol mandate-save mandate-cli mandate-wasm)
declare -A DEPS=(
  [mandate-core]=""
  [mandate-content]="mandate-core"
  [mandate-sim]="mandate-core mandate-content"
  [mandate-worldgen]="mandate-core mandate-content mandate-sim"
  [mandate-protocol]="mandate-core mandate-content mandate-sim"
  [mandate-save]="mandate-core mandate-content mandate-sim mandate-worldgen"
  [mandate-cli]="mandate-core mandate-content mandate-sim mandate-worldgen mandate-protocol mandate-save"
  [mandate-wasm]="mandate-core mandate-content mandate-sim mandate-worldgen mandate-protocol mandate-save"
)

changed_files() {
  { git diff --name-only "$BASE"...HEAD 2>/dev/null || git diff --name-only "$BASE" 2>/dev/null
    git diff --name-only HEAD 2>/dev/null
    git ls-files --others --exclude-standard 2>/dev/null; } | sort -u
}

run_affected() {
  run_fast
  local files; files="$(changed_files)"
  local -A hit=()
  local c
  if grep -qE '^(Cargo\.(toml|lock)|rust-toolchain\.toml|content/)' <<<"$files"; then
    for c in "${ALL_CRATES[@]}"; do hit[$c]=1; done
  fi
  for c in "${ALL_CRATES[@]}"; do
    grep -q "^crates/$c/" <<<"$files" && hit[$c]=1
  done
  local d
  for c in "${ALL_CRATES[@]}"; do
    for d in ${DEPS[$c]}; do [[ -n "${hit[$d]:-}" ]] && hit[$c]=1; done
  done
  local args=()
  for c in "${ALL_CRATES[@]}"; do [[ -n "${hit[$c]:-}" ]] && args+=(-p "$c"); done
  if ((${#args[@]})); then
    echo "affected crates: ${args[*]//-p /}"
    step cargo-test cargo test "${args[@]}"
  else
    echo "no Rust crates changed"
  fi
  if grep -q '^web/' <<<"$files"; then web_checks; else echo "web/ unchanged"; fi
}

web_checks() {
  [[ -d web/node_modules ]] || sh_step web-install 'cd web && npm ci'
  sh_step web-typecheck 'cd web && npm run typecheck'
  sh_step web-lint 'cd web && npm run lint'
  sh_step web-test 'cd web && npm test'
}

run_full() {
  step fmt cargo fmt --all --check
  step clippy cargo clippy --workspace --all-targets -- -D warnings
  step cargo-test cargo test --workspace
  step content-validate cargo run -q -p mandate-cli -- content validate
  sh_step generated 'cargo run -q -p mandate-cli -- content schema && git diff --exit-code content/schemas web/src/engine/protocol'
  step determinism cargo run -q --release -p mandate-cli -- determinism --seed 2026 --turns 80
  sh_step sweep 'cargo run -q --release -p mandate-cli -- sweep --seeds 8 --years 20 > /dev/null'
  sh_step bots 'cargo run -q --release -p mandate-cli -- bots --seeds 16 > target/check-logs/bots.json || { cat target/check-logs/bots.json; exit 1; }'
  # Report only, as in CI (non-blocking): the log is kept, the result is not counted.
  cargo run -q --release -p mandate-cli -- content doctor --seeds 8 --years 2 >"$LOGS/content-doctor.log" 2>&1 \
    && echo "INFO content-doctor ok (report only)" || echo "INFO content-doctor has findings (report only, $LOGS/content-doctor.log)"
  if [[ -f web/src/engine/pkg/mandate_wasm.js && -f target/wasm-node/mandate_wasm.js ]]; then
    sh_step wasm-parity 'N=$(cargo run -q --release -p mandate-cli -- run --seed 777 --size medium --turns 60 --hash-only) && node scripts/wasm-parity.mjs 777 60 "$N"'
  else
    echo "SKIP wasm-parity (build with scripts/build-wasm.sh first)"
  fi
  [[ -d web/node_modules ]] || sh_step web-install 'cd web && npm ci'
  sh_step web-typecheck 'cd web && npm run typecheck'
  sh_step web-lint 'cd web && npm run lint'
  sh_step web-test 'cd web && npm test'
  sh_step pixelkit 'cd tools/pixelkit && npm run check'
  sh_step web-build 'cd web && npm run build'
  if ((e2e)); then sh_step e2e 'cd web && npx playwright test --trace off'; else echo "SKIP e2e (pass --e2e)"; fi
}

case "$mode" in
  fast) run_fast ;;
  affected) run_affected ;;
  full) run_full ;;
esac

echo "---"
echo "$mode: $passed passed, ${#failed[@]} failed"
if ((${#failed[@]})); then
  echo "failed: ${failed[*]}"
  echo "logs: $LOGS/"
  exit 1
fi
exit 0
