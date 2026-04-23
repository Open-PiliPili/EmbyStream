#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CURRENT_DIR="$(pwd)"
OUTPUT_ROOT="${CURRENT_DIR}/.build"
TARGET=""
BUILD_WEB=1
PACKAGE=1
PROFILE_KIND="release"
PROFILE_DIR="release"
BINARY_OUTPUT_DIR=""
ARCHIVE_OUTPUT_PATH=""
COPIED_BINARY_PATH=""
SOURCE_BINARY_PATH=""

COLOR_BLUE=""
COLOR_CYAN=""
COLOR_GREEN=""
COLOR_RED=""
COLOR_YELLOW=""
COLOR_BOLD=""
COLOR_RESET=""

print_usage() {
  cat <<'EOF'
Build an EmbyStream binary, optionally embedding the Web Config Studio assets.

Usage:
  ./scripts/build-binary.sh [options]

Options:
  --target <triple>      Build for a specific Rust target triple
  --output-dir <dir>     Output root directory (default: ./.build)
  --debug                Build with cargo debug profile instead of release
  --no-web               Skip building web/dist before cargo build
  --no-package           Skip generating the .tar.gz archive
  -h, --help             Show this help text

Output layout:
  .build/binary/release/<artifact>
  .build/binary/debug/<artifact>

Examples:
  ./scripts/build-binary.sh
  ./scripts/build-binary.sh --target x86_64-unknown-linux-musl
  ./scripts/build-binary.sh --output-dir ./out --debug --no-package
EOF
}

setup_colors() {
  if [[ -t 1 ]] && [[ "${TERM:-}" != "dumb" ]]; then
    COLOR_BLUE=$'\033[34m'
    COLOR_CYAN=$'\033[36m'
    COLOR_GREEN=$'\033[32m'
    COLOR_RED=$'\033[31m'
    COLOR_YELLOW=$'\033[33m'
    COLOR_BOLD=$'\033[1m'
    COLOR_RESET=$'\033[0m'
  fi
}

print_status() {
  local color="$1"
  local label="$2"
  local message="$3"
  printf '%b%12s%b %s\n' "${color}${COLOR_BOLD}" "${label}" "${COLOR_RESET}" "${message}"
}

log_note() {
  print_status "${COLOR_CYAN}" "Preparing" "$1"
}

log_step() {
  print_status "${COLOR_BLUE}" "Building" "$1"
}

log_done() {
  print_status "${COLOR_GREEN}" "Finished" "$1"
}

log_warn() {
  print_status "${COLOR_YELLOW}" "Warning" "$1"
}

log_error() {
  print_status "${COLOR_RED}" "Error" "$1" >&2
}

fail() {
  log_error "$1"
  exit 1
}

resolve_path() {
  local path="$1"
  if [[ "${path}" = /* ]]; then
    printf '%s\n' "${path}"
  else
    printf '%s/%s\n' "${CURRENT_DIR}" "${path#./}"
  fi
}

require_option_value() {
  local flag="$1"
  local value="${2:-}"
  if [[ -z "${value}" ]]; then
    fail "missing value for ${flag}"
  fi
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --target)
        require_option_value "$1" "${2:-}"
        TARGET="$2"
        shift 2
        ;;
      --output-dir)
        require_option_value "$1" "${2:-}"
        OUTPUT_ROOT="$(resolve_path "$2")"
        shift 2
        ;;
      --debug)
        PROFILE_KIND="debug"
        PROFILE_DIR="debug"
        shift
        ;;
      --no-web)
        BUILD_WEB=0
        shift
        ;;
      --no-package)
        PACKAGE=0
        shift
        ;;
      -h|--help)
        print_usage
        exit 0
        ;;
      *)
        print_usage >&2
        fail "unknown option: $1"
        ;;
    esac
  done
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "required command not found: $1"
  fi
}

validate_environment() {
  require_command cargo
  require_command tar
  if [[ "${BUILD_WEB}" -eq 1 ]]; then
    require_command bun
  fi
}

artifact_name() {
  local name="embystream"
  if [[ -n "${TARGET}" ]]; then
    name="${name}-${TARGET}"
  fi
  printf '%s\n' "${name}"
}

prepare_output_paths() {
  local artifact
  artifact="$(artifact_name)"
  BINARY_OUTPUT_DIR="${OUTPUT_ROOT}/binary/${PROFILE_DIR}"
  COPIED_BINARY_PATH="${BINARY_OUTPUT_DIR}/${artifact}"
  ARCHIVE_OUTPUT_PATH="${BINARY_OUTPUT_DIR}/${artifact}.tar.gz"

  mkdir -p "${BINARY_OUTPUT_DIR}"
}

build_web_assets() {
  if [[ "${BUILD_WEB}" -ne 1 ]]; then
    log_warn "skipping web asset build"
    return
  fi

  log_step "web assets"
  (
    cd "${ROOT_DIR}/web"
    bun install --frozen-lockfile
    bun run build
  )
}

build_rust_binary() {
  local cargo_args
  cargo_args=(build)
  if [[ "${PROFILE_KIND}" == "release" ]]; then
    cargo_args+=(--release)
  fi
  if [[ -n "${TARGET}" ]]; then
    cargo_args+=(--target "${TARGET}")
  fi

  log_step "Rust binary (${PROFILE_KIND})"
  (
    cd "${ROOT_DIR}"
    cargo "${cargo_args[@]}"
  )
}

resolve_source_binary_path() {
  local binary_dir="${ROOT_DIR}/target"
  if [[ -n "${TARGET}" ]]; then
    binary_dir="${binary_dir}/${TARGET}"
  fi
  SOURCE_BINARY_PATH="${binary_dir}/${PROFILE_DIR}/embystream"

  if [[ ! -f "${SOURCE_BINARY_PATH}" ]]; then
    fail "built binary not found at ${SOURCE_BINARY_PATH}"
  fi
}

copy_binary_artifact() {
  log_note "copying binary artifact"
  cp "${SOURCE_BINARY_PATH}" "${COPIED_BINARY_PATH}"
  chmod +x "${COPIED_BINARY_PATH}"
}

package_binary_archive() {
  if [[ "${PACKAGE}" -ne 1 ]]; then
    log_warn "skipping binary archive packaging"
    return
  fi

  log_note "packaging binary archive"
  tar -czf "${ARCHIVE_OUTPUT_PATH}" -C "$(dirname "${SOURCE_BINARY_PATH}")" embystream
}

print_summary() {
  log_done "binary: ${COPIED_BINARY_PATH}"
  if [[ "${PACKAGE}" -eq 1 ]]; then
    log_done "archive: ${ARCHIVE_OUTPUT_PATH}"
  fi
}

main() {
  setup_colors
  parse_args "$@"
  validate_environment
  prepare_output_paths
  build_web_assets
  build_rust_binary
  resolve_source_binary_path
  copy_binary_artifact
  package_binary_archive
  print_summary
}

main "$@"
