set -euo pipefail

# Abort on any command failure with a clear message
trap 'rc=$?; echo -e "\nERROR: build_binaries.sh failed at line ${LINENO} (exit ${rc})" >&2; exit ${rc}' ERR

# ZISK_ETHPROOFS_BRANCH and ZEC_BRANCH are read from the zisk Cargo.toml after
# it is cloned (see load_branches_from_cargo).
ZISK_ETHPROOFS_BRANCH=""
ZEC_BRANCH=""

# Directory where the repos are cloned and the binaries are built.
WORK_DIR="/workspace"

usage() {
    cat <<EOF
Usage: $(basename "$0") --zisk-branch BRANCH [--work-dir DIR]

  --work-dir DIR         Directory where repos are cloned and built.
                         Default: ${WORK_DIR}
  -h, --help             Show this help.
EOF
}

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --zisk-branch) ZISK_BRANCH="$2"; shift 2 ;;
        --work-dir) WORK_DIR="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

# Resolve WORK_DIR to an absolute path (the build steps cd into subdirectories,
# so a relative path would break after the first cd).
mkdir -p "${WORK_DIR}"
WORK_DIR="$(cd "${WORK_DIR}" && pwd)"

# Cargo.toml (in the cloned zisk repo) providing the ethproofs / eth-client branches.
CARGO_TOML="${WORK_DIR}/zisk/Cargo.toml"

step() { echo -e "\n\033[1;32m==> $1\033[0m"; }

# read_cargo_value KEY: print the value of `KEY = "..."` from CARGO_TOML.
read_cargo_value() {
    local key="$1"
    sed -nE "s/^[[:space:]]*${key}[[:space:]]*=[[:space:]]*\"([^\"]*)\".*/\1/p" \
        "${CARGO_TOML}" | head -n1
}

# load_branches_from_cargo: read the ethproofs / eth-client branches from the
# zisk Cargo.toml (only available after zisk has been cloned).
load_branches_from_cargo() {
    step "Reading branches from ${CARGO_TOML}"

    if [[ ! -f "${CARGO_TOML}" ]]; then
        echo "ERROR: ${CARGO_TOML} not found" >&2
        exit 1
    fi

    ZISK_ETHPROOFS_BRANCH="$(read_cargo_value gha_zisk_ethproofs_branch)"
    ZEC_BRANCH="$(read_cargo_value gha_zisk_eth_client_branch)"

    if [[ -z "${ZISK_ETHPROOFS_BRANCH}" ]]; then
        echo "ERROR: gha_zisk_ethproofs_branch not found in ${CARGO_TOML}" >&2
        exit 1
    fi
    if [[ -z "${ZEC_BRANCH}" ]]; then
        echo "ERROR: gha_zisk_eth_client_branch not found in ${CARGO_TOML}" >&2
        exit 1
    fi

    echo "  zisk-ethproofs branch : ${ZISK_ETHPROOFS_BRANCH}"
    echo "  zisk-eth-client branch: ${ZEC_BRANCH}"
}

build_ethproofs() {
    step "Cloning zisk-ethproofs (${ZISK_ETHPROOFS_BRANCH})"
    cd "${WORK_DIR}"
    git clone --single-branch --depth 1 --branch "${ZISK_ETHPROOFS_BRANCH}" \
        https://github.com/0xPolygonHermez/zisk-ethproofs.git

    step "Building zisk-ethproofs"
    cd "${WORK_DIR}/zisk-ethproofs"
    source "${HOME}/.cargo/env"
    RUSTFLAGS='--cfg zisk_hints --cfg zisk_hints_metrics --cfg zisk_hints_single_thread' \
        cargo build --release
}

build_zec() {
    step "Cloning zisk-eth-client (${ZEC_BRANCH})"
    cd "${WORK_DIR}"
    git clone --single-branch --depth 1 --branch "${ZEC_BRANCH}" \
        https://github.com/0xPolygonHermez/zisk-eth-client.git

    step "Building zec-reth.elf"
    cd "${WORK_DIR}/zisk-eth-client/bin/guests/stateless-validator-reth"
    source "${HOME}/.cargo/env"
    "${HOME}/.zisk/bin/cargo-zisk" build --release
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
load_branches_from_cargo
build_ethproofs
build_zec
