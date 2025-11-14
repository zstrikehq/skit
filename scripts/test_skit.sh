#!/usr/bin/env bash
set -euo pipefail

# skit end-to-end CLI test runner
# - Runs all commands and parameter combinations non-interactively
# - Creates a temp workdir and HOME so no user state is touched
# - Can target a specific command subset: e.g., `scripts/test_skit.sh print`

usage() {
  cat <<'USAGE'
Usage: scripts/test_skit.sh [options] [test|all]

Options:
  --bin PATH           Path to skit binary (default: auto-detect or 'cargo run --')
  --keep-workdir       Do not clean up workdir after run
  --workdir PATH       Use/path for working directory (defaults to testsafes/<run>)
  -h, --help           Show this help

Tests:
  all (default), init, set, get, print, keys, env, exec, status, rm, rotate, ls, remember, cleanup, import, path-normalization

Examples:
  scripts/test_skit.sh
  scripts/test_skit.sh --bin ./target/release/skit
  scripts/test_skit.sh print
  scripts/test_skit.sh --keep-workdir rotate
USAGE
}

SKIT_BIN=""
KEEP_WORKDIR="false"
SELECTED_TEST="all"
WORKDIR_OPT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin)
      SKIT_BIN="${2:-}"
      shift 2
      ;;
    --keep-workdir)
      KEEP_WORKDIR="true"
      shift 1
      ;;
    --workdir)
      WORKDIR_OPT="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage; exit 0
      ;;
    all|init|set|get|print|keys|env|exec|status|rm|rotate|ls|remember|cleanup|import|path-normalization)
      SELECTED_TEST="$1"; shift 1
      ;;
    *)
      echo "Unknown arg: $1" >&2; usage; exit 2
      ;;
  esac
done

# Resolve skit binary
detect_skit_bin() {
  if [[ -n "${SKIT_BIN}" ]]; then
    echo "${SKIT_BIN}"; return 0
  fi
  if [[ -x ./target/debug/skit ]]; then
    echo ./target/debug/skit; return 0
  fi
  if [[ -x ./target/release/skit ]]; then
    echo ./target/release/skit; return 0
  fi
  # Fallback to cargo run
  echo "cargo run --quiet --"; return 0
}

SKIT_CMD="$(detect_skit_bin)"

# If SKIT_CMD points to a filesystem path (not cargo), make it absolute
if [[ "${SKIT_CMD}" != cargo* ]]; then
  case "${SKIT_CMD}" in
    /*) ;;
    *) SKIT_CMD="$(pwd)/${SKIT_CMD}" ;;
  esac
fi

run_skit() {
  # Allow callers to pass a leading literal 'skit' for readability; strip it.
  local args=("$@")
  if [[ "${#args[@]}" -gt 0 && "${args[0]}" == "skit" ]]; then
    args=("${args[@]:1}")
  fi
  # shellcheck disable=SC2086
  if [[ "${SKIT_CMD}" == cargo* ]]; then
    ${SKIT_CMD} "${args[@]}"
  else
    "${SKIT_CMD}" "${args[@]}"
  fi
}

# Resolve repository root (parent of this script dir)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Workdir defaults to testsafes/<run-id> under repo
RUN_ID="run-$(date +%Y%m%d-%H%M%S)-$RANDOM"
if [[ -n "${WORKDIR_OPT}" ]]; then
  WORKDIR="${WORKDIR_OPT}"
else
  WORKDIR="${REPO_ROOT}/testsafes/${RUN_ID}"
fi
mkdir -p "${WORKDIR}"

# HOME inside workdir so keys land in repo under testsafes
export HOME="${WORKDIR}/home"
mkdir -p "${HOME}/.config/skit/keys"

cleanup() {
  if [[ "${KEEP_WORKDIR}" != "true" ]]; then
    rm -rf "${WORKDIR}" || true
  else
    echo "Preserving WORKDIR=${WORKDIR}"
  fi
}
trap cleanup EXIT

cd "${WORKDIR}"

echo "Workdir: ${WORKDIR}"
echo "Home:    ${HOME} (under testsafes)"
echo "Using:   ${SKIT_CMD}"

# Test constants
SAFE=".env.safe"
SAFE2_NAME="myproj"       # normalizes to .myproj.safe
SAFE3_NAME="myproj.safe"  # normalizes to .myproj.safe
SAFE4_NAME=".alt.safe"    # kept as-is
PASS1="Aa1-_@#abcdEF"     # valid per policy
PASS2="Zz9-_@#qwerty"      # valid per policy
PASS3="Yy8-_@#asdfgh"      # valid per policy (for rotate)

# Helpers
assert_contains() {
  local haystack="$1" needle="$2"; shift 2
  echo "${haystack}" | grep -F -- "${needle}" >/dev/null || {
    echo "Assertion failed: output does not contain: ${needle}" >&2
    exit 1
  }
}

assert_rc() {
  local expected_rc="$1"; shift
  local cmd=("$@")
  set +e
  "${cmd[@]}"
  local rc=$?
  set -e
  if [[ "${rc}" -ne "${expected_rc}" ]]; then
    echo "Assertion failed: expected rc=${expected_rc}, got rc=${rc}: ${cmd[*]}" >&2
    exit 1
  fi
}

with_stdin() { # with_stdin "input" -- [skit] args...
  local input="$1"; shift
  local args=("$@")
  if [[ "${#args[@]}" -gt 0 && "${args[0]}" == "skit" ]]; then
    args=("${args[@]:1}")
  fi
  if [[ "${SKIT_CMD}" == cargo* ]]; then
    printf "%s" "${input}" | ${SKIT_CMD} "${args[@]}"
  else
    printf "%s" "${input}" | "${SKIT_CMD}" "${args[@]}"
  fi
}

step() { printf "\n=== %s ===\n" "$*"; }

safe_exists() {
  [[ -f "${SAFE}" ]]
}

prepare_safe() {
  if ! safe_exists; then
    step "prepare: init ${SAFE}"
    # Send empty line to trigger auto-generated password; --remember avoids further prompt
    with_stdin "
" skit --safe "${SAFE}" init --remember --description "Prepared Safe"
  fi
}

prepare_items() {
  prepare_safe
  step "prepare: set baseline keys"
  run_skit skit --safe "${SAFE}" set API_TOKEN secret123 || true
  run_skit skit --safe "${SAFE}" set -p LOG_LEVEL debug || true
}

test_init() {
  step "init: create ${SAFE} with remember + description (non-interactive)"
  with_stdin "${PASS1}
${PASS1}
" skit --safe "${SAFE}" init --remember --description "Test Safe"
  test -f "${SAFE}" || { echo "Missing ${SAFE}" >&2; exit 1; }

  step "init: idempotent re-run on existing safe"
  run_skit skit --safe "${SAFE}" init --remember --description "Test Safe"
}

test_set() {
  prepare_safe
  step "set: encrypted key requires auth (via remembered keyfile)"
  run_skit skit --safe "${SAFE}" set API_TOKEN secret123

  step "set: plain key"
  run_skit skit --safe "${SAFE}" set -p LOG_LEVEL debug

  step "set: invalid key should fail"
  assert_rc 1 run_skit skit --safe "${SAFE}" set 1BAD value || true
}

test_get() {
  prepare_items
  step "get: encrypted key"
  out=$(run_skit skit --safe "${SAFE}" get API_TOKEN)
  assert_contains "${out}" "secret123"

  step "get: plain key"
  out=$(run_skit skit --safe "${SAFE}" get LOG_LEVEL)
  assert_contains "${out}" "debug"

  step "get: missing key should fail"
  assert_rc 1 run_skit skit --safe "${SAFE}" get MISSING || true
}

test_print() {
  prepare_items
  step "print: default table"
  run_skit skit --safe "${SAFE}" --format table print

  step "print: --plain only"
  run_skit skit --safe "${SAFE}" --format table print --plain

  step "print: --enc only"
  run_skit skit --safe "${SAFE}" --format table print --enc

  step "print: conflicting flags should fail"
  assert_rc 1 run_skit skit --safe "${SAFE}" print --plain --enc || true

  step "print: json"
  run_skit skit --safe "${SAFE}" --format json print

  step "print: env"
  run_skit skit --safe "${SAFE}" --format env print

  step "print: terraform"
  run_skit skit --safe "${SAFE}" --format terraform print

  step "print: postman"
  run_skit skit --safe "${SAFE}" --format postman print
}

test_keys() {
  prepare_items
  step "keys: table"
  run_skit skit --safe "${SAFE}" --format table keys

  step "keys: json"
  run_skit skit --safe "${SAFE}" --format json keys
}

test_env() {
  prepare_items
  step "env: shell exports include keys"
  out=$(run_skit skit --safe "${SAFE}" env)
  assert_contains "${out}" "API_TOKEN"
  assert_contains "${out}" "LOG_LEVEL"
}

test_exec() {
  prepare_items
  step "exec: inject env and echo values"
  # Expect 'secret123|debug' in output
  if command -v sh >/dev/null 2>&1; then
    out=$(run_skit skit --safe "${SAFE}" exec sh -lc 'printf "%s|%s" "$API_TOKEN" "$LOG_LEVEL"')
  else
    out=$(run_skit skit --safe "${SAFE}" exec env)
  fi
  assert_contains "${out}" "secret123|debug" || true
}

test_status() {
  prepare_items
  step "status: table"
  run_skit skit --safe "${SAFE}" --format table status

  step "status: json"
  run_skit skit --safe "${SAFE}" --format json status
}

test_rm() {
  prepare_items
  step "rm: remove plain key"
  run_skit skit --safe "${SAFE}" rm LOG_LEVEL

  step "rm: remove missing key should fail"
  assert_rc 1 run_skit skit --safe "${SAFE}" rm NOPE || true
}

test_rotate() {
  prepare_items
  step "rotate: confirm, then set new password"
  with_stdin "yes
${PASS3}
${PASS3}
" skit --safe "${SAFE}" rotate

  step "remember-safekey with new password via env"
  SKIT_SAFEKEY="${PASS3}" run_skit skit --safe "${SAFE}" remember-safekey

  step "post-rotate get works with new key"
  out=$(run_skit skit --safe "${SAFE}" get API_TOKEN)
  assert_contains "${out}" "secret123"
}

test_ls() {
  prepare_safe
  step "ls: single safe in directory"
  run_skit skit --format table ls
  run_skit skit --format json ls

  step "ls: create second safe and list"
  with_stdin "${PASS2}
${PASS2}
" skit --safe "${SAFE4_NAME}" init --remember --description "Alt"
  run_skit skit --format table ls
}

test_remember() {
  step "remember-safekey with env var"
  SKIT_SAFEKEY="${PASS3}" run_skit skit --safe "${SAFE}" remember-safekey

  step "remember-safekey with prompt (no env)"
  # remove key file to force prompt
  KEYFILE=$(ls -1 "${HOME}/.config/skit/keys/"*.key | head -n1 || true)
  if [[ -n "${KEYFILE:-}" ]]; then rm -f "${KEYFILE}"; fi
  with_stdin "${PASS3}
" skit --safe "${SAFE}" remember-safekey
}

test_cleanup() {
  step "cleanup-keys: dry run (0 days => all keys)"
  run_skit skit cleanup-keys --older-than-days 0 --dry-run

  step "cleanup-keys: delete (confirm)"
  with_stdin "y
" skit cleanup-keys --older-than-days 0
}

test_import() {
  step "import: from .env-style file with plain-keys"
  cat > clear.env <<EOF
API_URL=https://example.local
PLAINTEXT=foo
SECRET_X=bar
EOF
  # Use PASS2 for new safe; answer 'y' to save key
  with_stdin "${PASS2}
y
" skit --safe ".imported.safe" import -f clear.env --plain-keys PLAINTEXT

  step "import: verify values"
  out=$(run_skit skit --safe ".imported.safe" get PLAINTEXT)
  assert_contains "${out}" "foo"
  out=$(run_skit skit --safe ".imported.safe" print --format env)
  assert_contains "${out}" "API_URL="
}

test_path_normalization() {
  step "path normalization: --safe ${SAFE2_NAME} => .myproj.safe"
  with_stdin "${PASS1}
${PASS1}
" skit --safe "${SAFE2_NAME}" init --remember --description "N1"
  test -f ".myproj.safe"

  step "path normalization: --safe ${SAFE3_NAME} => .myproj.safe"
  with_stdin "${PASS1}
${PASS1}
" skit --safe "${SAFE3_NAME}" init --remember --description "N2"
  test -f ".myproj.safe"
}

run_all() {
  test_init
  test_set
  test_get
  test_print
  test_keys
  test_env
  test_exec
  test_status
  test_rm
  test_rotate
  test_ls
  test_remember
  test_import
  test_path_normalization
  test_cleanup
}

case "${SELECTED_TEST}" in
  all) run_all ;;
  init) test_init ;;
  set) test_set ;;
  get) test_get ;;
  print) test_print ;;
  keys) test_keys ;;
  env) test_env ;;
  exec) test_exec ;;
  status) test_status ;;
  rm) test_rm ;;
  rotate) test_rotate ;;
  ls) test_ls ;;
  remember) test_remember ;;
  cleanup) test_cleanup ;;
  import) test_import ;;
  path-normalization) test_path_normalization ;;
esac

echo "\nAll selected tests passed."
