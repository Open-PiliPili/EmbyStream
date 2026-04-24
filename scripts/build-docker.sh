#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CURRENT_DIR="$(pwd)"
OUTPUT_ROOT="${CURRENT_DIR}/.build"
DOCKERFILE="Dockerfile"
TAGS=("embystream:latest")
PLATFORMS=""
PUSH=0
LOAD=1
DOCKER_OUTPUT_DIR=""
METADATA_PATH=""
TAGS_PATH=""
ARCHIVE_PATH=""
BUILD_ENGINE=""

COLOR_BLUE=""
COLOR_CYAN=""
COLOR_GREEN=""
COLOR_RED=""
COLOR_YELLOW=""
COLOR_BOLD=""
COLOR_RESET=""

print_usage() {
  cat <<'EOF'
Build an EmbyStream Docker image and write local artifacts under .build/docker.

Usage:
  ./scripts/build-docker.sh [options]

Options:
  --tag <name:tag>       Image tag to add; may be specified multiple times
  --platform <list>      Buildx platform list, e.g. linux/amd64,linux/arm64
  --output-dir <dir>     Output root directory (default: ./.build)
  --push                 Push the built image instead of loading it locally
  --no-load              Do not call --load for local builds
  -h, --help             Show this help text

Output layout:
  .build/docker/build-metadata.json
  .build/docker/tags.txt
  .build/docker/<tag>.tar   # local builds only

Examples:
  ./scripts/build-docker.sh --tag embystream:latest
  ./scripts/build-docker.sh --platform linux/amd64,linux/arm64 --push \
    --tag openpilipili/embystream:latest
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
      --tag)
        require_option_value "$1" "${2:-}"
        if [[ "${TAGS[*]}" == "embystream:latest" && "${#TAGS[@]}" -eq 1 ]]; then
          TAGS=()
        fi
        TAGS+=("$2")
        shift 2
        ;;
      --platform)
        require_option_value "$1" "${2:-}"
        PLATFORMS="$2"
        shift 2
        ;;
      --output-dir)
        require_option_value "$1" "${2:-}"
        OUTPUT_ROOT="$(resolve_path "$2")"
        shift 2
        ;;
      --push)
        PUSH=1
        LOAD=0
        shift
        ;;
      --no-load)
        LOAD=0
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
  require_command docker

  if [[ ! -f "${ROOT_DIR}/${DOCKERFILE}" ]]; then
    fail "dockerfile not found: ${DOCKERFILE}"
  fi

  if [[ -n "${PLATFORMS}" && "${PLATFORMS}" == *,* && "${PUSH}" -eq 0 && "${LOAD}" -eq 1 ]]; then
    fail "multi-platform builds cannot use --load; use --push or --no-load"
  fi

  determine_build_engine
  ensure_docker_daemon
}

determine_build_engine() {
  local needs_buildx=0
  local has_buildx=0

  if docker buildx version >/dev/null 2>&1; then
    has_buildx=1
  fi

  if [[ "${PUSH}" -eq 1 ]]; then
    needs_buildx=1
  fi

  if [[ -n "${PLATFORMS}" && "${PLATFORMS}" == *,* ]]; then
    needs_buildx=1
  fi

  if [[ "${needs_buildx}" -eq 1 ]]; then
    if [[ "${has_buildx}" -eq 1 ]]; then
      BUILD_ENGINE="buildx"
      return
    fi
    fail "docker buildx is required for --push or multi-platform builds"
  fi

  if [[ "${has_buildx}" -eq 1 ]]; then
    BUILD_ENGINE="buildx"
  else
    BUILD_ENGINE="build"
  fi
}

ensure_docker_daemon() {
  if docker info >/dev/null 2>&1; then
    return
  fi

  fail "docker daemon is not available; start Docker Desktop or ensure /var/run/docker.sock is reachable"
}

sanitize_tag() {
  printf '%s' "$1" | tr '/:@' '_' | tr -c 'A-Za-z0-9._-' '_'
}

prepare_output_paths() {
  local primary_tag
  primary_tag="${TAGS[0]}"
  DOCKER_OUTPUT_DIR="${OUTPUT_ROOT}/docker"
  METADATA_PATH="${DOCKER_OUTPUT_DIR}/build-metadata.json"
  TAGS_PATH="${DOCKER_OUTPUT_DIR}/tags.txt"
  ARCHIVE_PATH="${DOCKER_OUTPUT_DIR}/$(sanitize_tag "${primary_tag}").tar"

  mkdir -p "${DOCKER_OUTPUT_DIR}"
}

write_tags_manifest() {
  log_note "writing docker tag manifest"
  printf '%s\n' "${TAGS[@]}" > "${TAGS_PATH}"
}

build_image() {
  local cmd

  if [[ "${BUILD_ENGINE}" == "buildx" ]]; then
    cmd=(docker buildx build --file "${ROOT_DIR}/${DOCKERFILE}")
  else
    cmd=(docker build --file "${ROOT_DIR}/${DOCKERFILE}")
  fi

  for tag in "${TAGS[@]}"; do
    cmd+=(--tag "${tag}")
  done

  if [[ -n "${PLATFORMS}" ]]; then
    cmd+=(--platform "${PLATFORMS}")
  fi

  if [[ "${PUSH}" -eq 1 ]]; then
    cmd+=(--push)
  elif [[ "${LOAD}" -eq 1 ]]; then
    if [[ "${BUILD_ENGINE}" == "buildx" ]]; then
      cmd+=(--load)
    fi
  fi

  cmd+=("${ROOT_DIR}")

  log_step "docker image (${BUILD_ENGINE})"
  "${cmd[@]}"
}

json_escape() {
  local value="$1"
  value=${value//\\/\\\\}
  value=${value//\"/\\\"}
  value=${value//$'\n'/\\n}
  value=${value//$'\r'/\\r}
  value=${value//$'\t'/\\t}
  printf '%s' "${value}"
}

join_json_string_array() {
  local first=1
  local item

  for item in "$@"; do
    if [[ "${first}" -eq 0 ]]; then
      printf ','
    fi
    first=0
    printf '"%s"' "$(json_escape "${item}")"
  done
}

build_image_ids_json() {
  local first=1
  local tag image_id

  for tag in "${TAGS[@]}"; do
    image_id="$(docker image inspect --format '{{.Id}}' "${tag}" 2>/dev/null || true)"
    if [[ -z "${image_id}" ]]; then
      continue
    fi

    if [[ "${first}" -eq 0 ]]; then
      printf ',\n'
    fi
    first=0
    printf '    "%s": "%s"' "$(json_escape "${tag}")" "$(json_escape "${image_id}")"
  done
}

write_build_metadata() {
  local image_ids_json=""

  if [[ "${LOAD}" -eq 1 ]]; then
    image_ids_json="$(build_image_ids_json)"
  fi

  log_note "writing docker build metadata"
  cat > "${METADATA_PATH}" <<EOF
{
  "dockerfile": "$(json_escape "${DOCKERFILE}")",
  "engine": "$(json_escape "${BUILD_ENGINE}")",
  "output_dir": "$(json_escape "${DOCKER_OUTPUT_DIR}")",
  "platforms": "$(json_escape "${PLATFORMS}")",
  "push": ${PUSH},
  "load": ${LOAD},
  "tags": [$(join_json_string_array "${TAGS[@]}")],
  "image_ids": {
${image_ids_json}
  }
}
EOF
}

export_local_image_archive() {
  if [[ "${LOAD}" -ne 1 ]]; then
    log_warn "skipping local docker archive export because --load is disabled"
    return
  fi

  log_note "exporting local docker image archive"
  docker save -o "${ARCHIVE_PATH}" "${TAGS[@]}"
}

print_summary() {
  log_done "metadata: ${METADATA_PATH}"
  log_done "tags: ${TAGS_PATH}"
  if [[ "${LOAD}" -eq 1 ]]; then
    log_done "archive: ${ARCHIVE_PATH}"
  fi
}

main() {
  setup_colors
  parse_args "$@"
  validate_environment
  prepare_output_paths
  write_tags_manifest
  build_image
  write_build_metadata
  export_local_image_archive
  print_summary
}

main "$@"
