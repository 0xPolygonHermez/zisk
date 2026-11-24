#!/bin/bash
#
# Download the setup built (or cached) by the "Test on Ubuntu x86_64" job of a
# PR's last CI run whose "Build setup" step succeeded. The worker and the cache
# entry (/home/gha/cache-setup/<platform>/<key>) are read from the job log.
#
# By default the setup is packed on the worker (in /home/gha/cache-setup/temp,
# removed at the end) and downloaded as DIR/<name>.tar.gz, which unpacks to
# <name>/. With --no-compress it is copied as is to DIR/<name>/.
#
# Usage: fetch_pr_setup.sh [options] <PR number>
#   -o, --output DIR   Destination directory (default: current directory).
#   -n, --name NAME    Local name of the setup (default: pr<PR>-<key>,
#                      e.g. pr1510-c49efb02-blake3).
#   -H, --host HOST    ssh host to copy from (default: the job's worker-N).
#       --no-compress  Copy the setup directory as is instead of a tar.gz.
#   -f, --force        Replace the local copy if it exists.
#   -h, --help         Show this help.
#
# Environment:
#   GITHUB_TOKEN / GH_TOKEN  Needed to read job logs, even on a public repo
#                            (falls back to `gh auth token`).
#   ZISK_GH_REPO             Repository (default: 0xPolygonHermez/zisk).
#   SETUP_HOST               Same as --host.
#
# Needs curl, jq, ssh access to the worker and rsync (or scp).

set -uo pipefail

REPO="${ZISK_GH_REPO:-0xPolygonHermez/zisk}"
GH_API="https://api.github.com/repos/${REPO}"
WORKFLOW_FILE="pr.yml"
JOB_NAME="Test on Ubuntu x86_64"
SETUP_STEP_PREFIX="Build setup"
# The worker's setup cache as mounted in the CI container
CONTAINER_CACHE_DIR="/home/ziskuser/output"
WORKER_CACHE_DIR="/home/gha/cache-setup"
MAX_RUNS=100
# tar progress every TAR_CHECKPOINT records of 10 KiB (~100 MiB)
TAR_CHECKPOINT=10000
TAR_RECORD_SIZE=10240

usage() {
    sed -n '/^# Usage:/,/^# Needs/p' "$0" | sed 's/^# \{0,1\}//'
}

# ----- messages ----------------------------------------------------------------------

if [[ -t 1 ]]; then
    BOLD=$'\033[1m' GREEN=$'\033[32m' RED=$'\033[31m' YELLOW=$'\033[33m' RESET=$'\033[0m'
else
    BOLD="" GREEN="" RED="" YELLOW="" RESET=""
fi

info()    { echo "$1"; }
warn()    { echo "${BOLD}${YELLOW}🚨  $1${RESET}"; }
success() { echo "${BOLD}${GREEN}✅ $1${RESET}"; }

die() {
    printf '\n%s\n' "${RED}❌ Error: $1${RESET}" >&2
    exit 1
}

# ----- time and size formatting ------------------------------------------------------

fmt_secs() {
    local s=$1
    if (( s >= 3600 )); then printf '%dh%02dm%02ds' $((s / 3600)) $((s % 3600 / 60)) $((s % 60))
    elif (( s >= 60 )); then printf '%dm%02ds' $((s / 60)) $((s % 60))
    else printf '%ds' "$s"; fi
}

fmt_bytes() {
    numfmt --to=iec --suffix=B --format='%.1f' "$1" 2>/dev/null || printf '%sB' "$1"
}

# ----- phases ------------------------------------------------------------------------

PHASE_START=0 CURRENT_PHASE=0 TOTAL_PHASES=0
phase() {
    PHASE_START=$EPOCHSECONDS
    CURRENT_PHASE=$((CURRENT_PHASE + 1))
    echo "${BOLD}${GREEN}[${CURRENT_PHASE}/${TOTAL_PHASES}] $1${RESET}"
}

phase_done() {
    info "  ↳ done in $(fmt_secs $((EPOCHSECONDS - PHASE_START)))"
}

# Turn tar checkpoint lines ("tar: N") into a progress line against $1 bytes.
tar_progress() {
    local total=$1 start=$EPOCHSECONDS last_pct=-5
    local line n bytes pct elapsed rate eta
    (( total > 0 )) || total=1
    while IFS= read -r line; do
        n="${line##*: }"
        if [[ "$line" != tar:* || ! "$n" =~ ^[0-9]+$ ]]; then
            printf '\n%s\n' "$line" >&2
            continue
        fi
        bytes=$(( n * TAR_RECORD_SIZE ))
        (( bytes > total )) && bytes=$total
        pct=$(( bytes * 100 / total ))
        elapsed=$(( EPOCHSECONDS - start ))
        rate=0 eta="?"
        if (( elapsed > 0 )); then
            rate=$(( bytes / elapsed ))
            (( rate > 0 )) && eta="$(fmt_secs $(( (total - bytes) / rate )))"
        fi
        if [[ -t 1 ]]; then
            printf '\r  %3d%%  %s / %s  %s/s  ETA %s\033[K' "$pct" \
                "$(fmt_bytes "$bytes")" "$(fmt_bytes "$total")" "$(fmt_bytes "$rate")" "$eta"
        elif (( pct >= last_pct + 5 )); then
            printf '  %3d%%  %s / %s  %s/s  ETA %s\n' "$pct" \
                "$(fmt_bytes "$bytes")" "$(fmt_bytes "$total")" "$(fmt_bytes "$rate")" "$eta"
            last_pct=$pct
        fi
    done
    [[ -t 1 ]] && printf '\n'
    return 0
}

# ----- GitHub API --------------------------------------------------------------------

GH_TOKEN_VALUE=""
gh_api() {
    local auth=()
    [[ -n "$GH_TOKEN_VALUE" ]] && auth=(-H "Authorization: Bearer ${GH_TOKEN_VALUE}")
    # -L: the log endpoint redirects to a signed URL
    curl -fsSL --retry 3 -H "Accept: application/vnd.github+json" \
        -H "X-GitHub-Api-Version: 2022-11-28" "${auth[@]}" "${GH_API}/$1"
}

resolve_token() {
    GH_TOKEN_VALUE="${GITHUB_TOKEN:-${GH_TOKEN:-}}"
    if [[ -z "$GH_TOKEN_VALUE" ]] && command -v gh >/dev/null; then
        GH_TOKEN_VALUE="$(gh auth token 2>/dev/null)" || GH_TOKEN_VALUE=""
    fi
    [[ -n "$GH_TOKEN_VALUE" ]] || die "a GitHub token is needed to read the job log: set GITHUB_TOKEN (or log in with gh)"
}

# ----- cleanup -----------------------------------------------------------------------

# Remove the temporary tarballs and any half-copied setup
REMOTE_HOST="" REMOTE_TAR="" LOCAL_TAR="" INCOMPLETE_DIR=""
cleanup() {
    local status=$?
    trap - EXIT INT TERM
    if [[ -n "$INCOMPLETE_DIR" && -d "$INCOMPLETE_DIR" ]]; then
        warn "Removing the incomplete ${INCOMPLETE_DIR}"
        rm -rf "$INCOMPLETE_DIR"
    fi
    [[ -n "$LOCAL_TAR" ]] && rm -f "$LOCAL_TAR"
    if [[ -n "$REMOTE_TAR" ]]; then
        info "Removing ${REMOTE_HOST}:${REMOTE_TAR}..."
        ssh "$REMOTE_HOST" "rm -f '${REMOTE_TAR}'" \
            || warn "cannot remove ${REMOTE_HOST}:${REMOTE_TAR}, remove it by hand"
    fi
    exit "$status"
}

main() {
    local pr="" dest="." name="" host="${SETUP_HOST:-}" force=0 compress=1
    local start=$EPOCHSECONDS

    while [[ $# -gt 0 ]]; do
        case "$1" in
            -o|--output)      [[ $# -ge 2 ]] || die "$1 needs a value"; dest="$2"; shift 2 ;;
            -n|--name)        [[ $# -ge 2 ]] || die "$1 needs a value"; name="$2"; shift 2 ;;
            -H|--host)        [[ $# -ge 2 ]] || die "$1 needs a value"; host="$2"; shift 2 ;;
            --no-compress)    compress=0; shift ;;
            -f|--force)       force=1; shift ;;
            -h|--help)        usage; exit 0 ;;
            -*)               usage >&2; die "unknown option: $1" ;;
            *)
                [[ -z "$pr" ]] || die "only one PR number may be given"
                pr="$1"; shift ;;
        esac
    done

    [[ -n "$pr" ]] || { usage >&2; die "missing PR number"; }
    [[ "$pr" =~ ^[0-9]+$ ]] || die "PR number must be numeric, got '$pr'"
    command -v curl >/dev/null || die "curl is not installed"
    command -v jq >/dev/null || die "jq is not installed"
    [[ -d "$dest" ]] || die "destination directory does not exist: $dest"
    name="${name%.tar.gz}"
    if [[ -n "$name" ]]; then
        [[ "$name" =~ ^[A-Za-z0-9._-]+$ && "$name" != .* ]] \
            || die "--name must be a plain file name (letters, digits, . _ -), got '$name'"
    fi
    resolve_token

    TOTAL_PHASES=4
    [[ "$compress" -eq 1 ]] && TOTAL_PHASES=5

    trap cleanup EXIT
    trap 'exit 130' INT TERM

    # ----- run whose setup step succeeded ---------------------------------------------
    phase "Finding the last CI run of PR #${pr} with a successful setup step..."
    local pr_json branch head_owner
    pr_json="$(gh_api "pulls/${pr}")" || die "cannot read PR #${pr} of ${REPO}"
    branch="$(jq -r .head.ref <<< "$pr_json")"
    head_owner="$(jq -r .head.repo.owner.login <<< "$pr_json")"
    info "  PR #${pr}: $(jq -r .title <<< "$pr_json") (${head_owner}:${branch})"

    # Newest first; the owner check skips forks with the same branch name
    local run_ids
    run_ids="$(gh_api "actions/workflows/${WORKFLOW_FILE}/runs?event=pull_request&branch=$(jq -rn --arg b "$branch" '$b|@uri')&per_page=${MAX_RUNS}" |
            jq -r --arg owner "$head_owner" '.workflow_runs[] | select(.head_repository.owner.login == $owner) | .id')" \
        || die "cannot list the workflow runs of branch ${branch}"
    [[ -n "$run_ids" ]] || die "no '${WORKFLOW_FILE}' runs found for PR #${pr}"

    local run_id="" job_id="" setup_step="" job checked=0
    for id in $run_ids; do
        checked=$((checked + 1))
        job="$(gh_api "actions/runs/${id}/jobs?per_page=100" | jq -c --arg name "$JOB_NAME" --arg step "$SETUP_STEP_PREFIX" '
            first(.jobs[] | select(.name == $name)
                | {id, step: (first(.steps[]? | select((.name | startswith($step)) and .conclusion == "success")) | .name)}
                | select(.step != null))')" || continue
        if [[ -n "$job" ]]; then
            run_id="$id"
            job_id="$(jq -r .id <<< "$job")"
            setup_step="$(jq -r .step <<< "$job")"
            break
        fi
    done
    [[ -n "$run_id" ]] || die "none of the ${checked} runs of PR #${pr} has a successful '${SETUP_STEP_PREFIX}' step in '${JOB_NAME}'"
    info "  Run: https://github.com/${REPO}/actions/runs/${run_id} (job ${job_id}, step '${setup_step}')"
    phase_done

    # ----- worker, hash and cache entry from the job log -------------------------------
    phase "Reading the job log..."
    local log machine hash cache_path entry key hash_mode
    log="$(gh_api "actions/jobs/${job_id}/logs")" \
        || die "cannot download the log of job ${job_id} (expired, or the token has no access?)"

    machine="$(grep -oP "Machine name: '\K[^']+" <<< "$log" | head -n1)"
    [[ "$machine" =~ ^worker-([0-9]+)$ ]] || die "cannot find the worker in the job log (Machine name: '${machine}')"

    hash="$(grep -oP 'local input hash: \K[0-9a-f]{64}' <<< "$log" | tail -n1)"
    [[ -n "$hash" ]] || die "cannot find 'local input hash' in the job log"

    # Printed on both hit and miss: the entry exists on the worker either way
    cache_path="$(grep -oP '==> cache (hit|miss): \K\S+' <<< "$log" | tail -n1)"
    [[ "$cache_path" == "${CONTAINER_CACHE_DIR}/"* ]] \
        || die "cannot find the setup cache entry in the job log (got '${cache_path}')"
    entry="${cache_path#"${CONTAINER_CACHE_DIR}/"}"   # <platform>/<key>
    key="${entry##*/}"
    [[ "$entry" =~ ^[a-z0-9]+/[a-z0-9-]+$ ]] || die "unexpected cache entry '${entry}'"

    # Cross-check the entry with the hash and the step's hash mode
    hash_mode="$(sed -n 's/.*(\(.*\)).*/\1/p' <<< "$setup_step" | tr '[:upper:]' '[:lower:]')"
    [[ "$key" == "${hash:0:4}${hash: -4}-${hash_mode}" ]] \
        || die "cache entry '${key}' does not match hash ${hash} (mode '${hash_mode}')"

    host="${host:-$machine}"
    name="${name:-pr${pr}-${key}}"
    local remote_dir="${WORKER_CACHE_DIR}/${entry}"
    local target="${dest%/}/${name}"
    [[ "$compress" -eq 1 ]] && target="${target}.tar.gz"

    info "  Worker:     ${machine}"
    info "  Hash:       ${hash} (${hash_mode})"
    info "  Setup:      ${host}:${remote_dir}"
    info "  Local copy: ${target}"
    phase_done

    # ----- worker and destination checks ------------------------------------------------
    phase "Checking ${host} and the destination..."
    if [[ -e "$target" ]]; then
        [[ "$force" -eq 1 ]] || die "${target} already exists (use --force to replace it)"
        warn "Removing existing ${target} (--force)"
        rm -rf "$target" || die "cannot remove ${target}"
    fi

    local remote_status remote_rsync remote_pigz total_bytes
    remote_status="$(ssh "$host" "
        if [ ! -d '${remote_dir}/provingKey' ]; then echo missing; exit 0; fi
        echo size=\$(du -sb '${remote_dir}' | cut -f1)
        command -v rsync >/dev/null 2>&1 && echo rsync=1 || echo rsync=0
        command -v pigz  >/dev/null 2>&1 && echo pigz=1  || echo pigz=0")" \
        || die "cannot connect to ${host} over ssh"
    [[ "$remote_status" != "missing" ]] \
        || die "${host}:${remote_dir}/provingKey does not exist (cache entry removed?)"
    total_bytes="$(sed -n 's/^size=//p' <<< "$remote_status")"
    remote_rsync="$(grep -c '^rsync=1$' <<< "$remote_status")"
    remote_pigz="$(grep -c '^pigz=1$' <<< "$remote_status")"
    info "  Setup size: $(fmt_bytes "$total_bytes")"

    local use_rsync=0
    if ! command -v rsync >/dev/null; then
        warn "rsync is not installed locally, falling back to scp"
    elif [[ "$remote_rsync" -ne 1 ]]; then
        warn "rsync is not installed on ${host}, falling back to scp"
    else
        use_rsync=1
    fi
    phase_done

    REMOTE_HOST="$host"

    if [[ "$compress" -eq 0 ]]; then
        # ----- direct copy into DIR/<name>/ ------------------------------------------------
        INCOMPLETE_DIR="$target"
        if [[ "$use_rsync" -eq 1 ]]; then
            phase "Copying the setup with rsync..."
            # --no-inc-recursive: progress over the whole setup, not per batch
            rsync -a --no-inc-recursive --info=progress2 "${host}:${remote_dir}/" "${target}/" \
                || die "rsync from ${host} failed"
        else
            phase "Copying the setup with scp..."
            scp -r "${host}:${remote_dir}" "$target" || die "scp from ${host} failed"
        fi
        [[ -d "${target}/provingKey" ]] || die "the copied setup has no provingKey/"
        INCOMPLETE_DIR=""
        phase_done
    else
        # ----- pack on the worker --------------------------------------------------------
        local stamp rand compressor="gzip"
        stamp="$(date +%Y%m%d-%H%M%S)"
        rand="$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
        REMOTE_TAR="${WORKER_CACHE_DIR}/temp/${key}-${stamp}-${rand}.tar.gz"
        [[ "$remote_pigz" -eq 1 ]] && compressor="pigz"

        phase "Packing the setup on ${host} (${compressor}) into ${REMOTE_TAR}..."
        # tar's checkpoints come back on ssh's stderr; top dir renamed to <name>
        ssh "$host" bash -s -- "$WORKER_CACHE_DIR" "${entry%/*}" "$key" "$REMOTE_TAR" \
            "$compressor" "$TAR_CHECKPOINT" "$name" <<'EOF' 2>&1 >/dev/null | tar_progress "$total_bytes"
set -o pipefail
cache_dir=$1 platform=$2 key=$3 out=$4 compressor=$5 checkpoint=$6 name=$7
mkdir -p "$cache_dir/temp" || exit 1
tar -C "$cache_dir/$platform" -cf - "$key" --transform="s|^${key}|${name}|" \
    --checkpoint="$checkpoint" --checkpoint-action=echo='%u' | "$compressor" > "$out"
EOF
        [[ "${PIPESTATUS[0]}" -eq 0 ]] || die "cannot pack the setup on ${host}"
        local tar_bytes
        tar_bytes="$(ssh "$host" "stat -c %s '${REMOTE_TAR}'")" || die "cannot read the size of ${REMOTE_TAR}"
        info "  Tarball: $(fmt_bytes "$tar_bytes") ($(( tar_bytes * 100 / total_bytes ))% of the setup)"
        phase_done

        # ----- download ----------------------------------------------------------------
        # To a hidden .part file, renamed only once complete
        LOCAL_TAR="${dest%/}/.${name}.tar.gz.part"
        if [[ "$use_rsync" -eq 1 ]]; then
            phase "Downloading the tarball with rsync..."
            rsync --partial --info=progress2 "${host}:${REMOTE_TAR}" "$LOCAL_TAR" \
                || die "rsync from ${host} failed"
        else
            phase "Downloading the tarball with scp..."
            scp "${host}:${REMOTE_TAR}" "$LOCAL_TAR" || die "scp from ${host} failed"
        fi
        [[ "$(stat -c %s "$LOCAL_TAR")" == "$tar_bytes" ]] \
            || die "the downloaded tarball size does not match ${host}:${REMOTE_TAR}"
        mv "$LOCAL_TAR" "$target" || die "cannot rename ${LOCAL_TAR} to ${target}"
        LOCAL_TAR=""
        phase_done
    fi

    success "Setup ${key} of PR #${pr} copied to ${target} in $(fmt_secs $((EPOCHSECONDS - start)))"
}

main "$@"
