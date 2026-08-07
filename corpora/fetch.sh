#!/usr/bin/env bash
#
# Fetch the test corpora described by corpora/sources/*.sources.
#
# MASTER_PLAN §2: "test corpora as git submodules / fetch scripts".
# MASTER_PLAN §10: "corpus files respect upstream licenses (fetch scripts, not
#                   vendored copies, where required)."
#
# Nothing fetched here is ever committed. corpora/.gitignore enforces that.
#
# Properties this script guarantees, because the proofs downstream depend on
# them:
#
#   * Pinned. Every source is fetched at a full 40-char commit SHA. A branch or
#     a tag would make a green proof mean "green against whatever upstream
#     looked like this morning".
#   * Capped, exactly. Each source contributes exactly `max_files` files, in
#     include-pattern order then byte-wise sorted order. A short yield is a hard
#     error: the rev is pinned, so the count is a fact, not an estimate.
#   * Attributed. The upstream LICENSE file is copied next to the fetched files
#     and its SHA-256 recorded, so a licence change upstream is visible as a
#     diff rather than a surprise.
#   * Manifested. Every fetched file's SHA-256 goes into MANIFEST.tsv, sorted.
#     That file is the receipt: it says exactly which bytes a proof ran against.
#
# Usage:
#   corpora/fetch.sh                 # fetch every category
#   corpora/fetch.sh helm-charts     # fetch one category
#   DRY_RUN=1 corpora/fetch.sh       # print the plan, touch nothing

set -euo pipefail

# Byte-wise, locale-independent sorting everywhere (MASTER_PLAN §9.5).
export LC_ALL=C

CORPORA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCES_DIR="$CORPORA_DIR/sources"
DRY_RUN="${DRY_RUN:-0}"

die() { printf 'fetch.sh: %s\n' "$*" >&2; exit 1; }
note() { printf '  %s\n' "$*" >&2; }

# Trim with parameter expansion, not `xargs`: xargs does quote and backslash
# processing, which would mangle glob patterns like `charts/*/templates/*.yaml`.
trim() { local v="$1"; v="${v#"${v%%[![:space:]]*}"}"; printf '%s' "${v%"${v##*[![:space:]]}"}"; }

command -v git >/dev/null || die "git is required"
command -v sha256sum >/dev/null || SHA_CMD="shasum -a 256"
SHA_CMD="${SHA_CMD:-sha256sum}"

sha_of() { $SHA_CMD "$1" | awk '{print $1}'; }

# Temp checkouts are tracked and removed on exit. A RETURN trap would be wrong
# here: bash keeps a RETURN trap installed after the function that set it
# returns, so it would fire again in an unrelated scope where $work is unset.
WORKDIRS=()
cleanup() {
  local d
  for d in ${WORKDIRS+"${WORKDIRS[@]}"}; do rm -rf "$d"; done
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Fetch one [source] record into $dest_root/$name/
# ---------------------------------------------------------------------------
fetch_source() {
  local dest_root="$1" name="$2" repo="$3" rev="$4" license="$5" license_file="$6"
  local max_files="$7"; shift 7
  local includes=("$@")

  if [[ ! "$rev" =~ ^[0-9a-f]{40}$ ]]; then
    die "$name: rev '$rev' is not a 40-char lowercase commit SHA"
  fi

  note "$name  <-  $repo @ ${rev:0:12}  (<=$max_files files, $license)"
  if [[ "$DRY_RUN" == "1" ]]; then
    local pattern
    for pattern in "${includes[@]}"; do note "    include $pattern"; done
    return 0
  fi

  local dest="$dest_root/$name"
  local work; work="$(mktemp -d)"
  WORKDIRS+=("$work")

  # Partial clone: metadata only, blobs on demand. A full clone of bitnami/charts
  # is gigabytes; this is tens of megabytes.
  git -C "$work" init --quiet
  git -C "$work" remote add origin "$repo"
  if ! git -C "$work" fetch --quiet --depth 1 --filter=blob:none origin "$rev"; then
    die "$name: cannot fetch $rev from $repo (was the commit force-pushed away?)"
  fi
  git -C "$work" checkout --quiet FETCH_HEAD

  [[ -f "$work/$license_file" ]] || die "$name: declared license_file '$license_file' is not in the repo"

  rm -rf "$dest"
  mkdir -p "$dest"
  cp "$work/$license_file" "$dest/UPSTREAM-LICENSE"

  # Candidates are gathered per include pattern, each pattern's matches sorted
  # byte-wise, patterns kept in the order they are written. Dedupe preserves
  # first occurrence, so the manifest's pattern order IS the selection priority
  # — and the whole thing stays deterministic: same rev + same caps => same
  # corpus, on any machine.
  local candidates=()
  for pattern in "${includes[@]}"; do
    while IFS= read -r match; do
      candidates+=("$match")
    done < <(cd "$work" && shopt -s globstar nullglob && printf '%s\n' $pattern | sort)
  done

  local manifest="$dest/MANIFEST.tsv"
  {
    printf '#! source = %s\n' "$name"
    printf '#! repo = %s\n' "$repo"
    printf '#! rev = %s\n' "$rev"
    printf '#! license = %s\n' "$license"
    printf '#! upstream_license_sha256 = %s\n' "$(sha_of "$dest/UPSTREAM-LICENSE")"
  } > "$manifest"

  local count=0 rel
  while IFS= read -r rel; do
    [[ -f "$work/$rel" ]] || continue
    (( count < max_files )) || break
    mkdir -p "$dest/files/$(dirname "$rel")"
    cp "$work/$rel" "$dest/files/$rel"
    printf '%s\t%s\n' "$(sha_of "$work/$rel")" "$rel" >> "$manifest"
    count=$((count + 1))
  done < <(printf '%s\n' "${candidates[@]}" | awk 'NF && !seen[$0]++')

  note "    $count file(s)"
  # The rev is pinned, so the yield is a deterministic fact, not an estimate.
  # Asserting it exactly is what lets corpora/README.md say "1,000 files" and
  # mean it. A short yield means the include patterns or max_files are wrong.
  (( count == max_files )) || die "$name: yielded $count file(s) but max_files is \
$max_files. The rev is pinned, so this number is deterministic — widen the include \
patterns, or lower max_files and the category cap with it."
  rm -rf "$work"
}

# ---------------------------------------------------------------------------
# Parse one category manifest and fetch each [source] in it
# ---------------------------------------------------------------------------
fetch_category() {
  local manifest="$1"
  local name="" repo="" rev="" license="" license_file="" max_files=""
  local includes=()
  local have_source=0

  # Pre-scan the header so `dest_root` is known before the first [source] is
  # flushed. Reading it in the main loop would leave `dest_root` empty for every
  # source but the last, and files would land outside corpora/.
  local category cap
  category="$(trim "$(sed -n 's/^#![[:space:]]*category[[:space:]]*=//p' "$manifest" | head -1)")"
  cap="$(trim "$(sed -n 's/^#![[:space:]]*cap[[:space:]]*=//p' "$manifest" | head -1)")"
  [[ -n "$category" ]] || die "$manifest: missing '#! category ='"
  [[ -n "$cap" ]] || die "$manifest: missing '#! cap ='"

  local dest_root="$CORPORA_DIR/$category"
  [[ "$DRY_RUN" == "1" ]] || mkdir -p "$dest_root"
  printf '%s (cap %s)\n' "$category" "$cap" >&2

  flush() {
    (( have_source )) || return 0
    fetch_source "$dest_root" "$name" "$repo" "$rev" "$license" "$license_file" \
      "$max_files" "${includes[@]}"
    name=""; repo=""; rev=""; license=""; license_file=""; max_files=""
    includes=(); have_source=0
  }

  local line key value
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line#"${line%%[![:space:]]*}"}"   # ltrim
    [[ -z "$line" ]] && continue
    if [[ "$line" == '#!'* ]]; then
      # Header directives were read by the pre-scan above.
      continue
    fi
    [[ "$line" == '#'* ]] && continue
    if [[ "$line" == '[source]' ]]; then
      flush
      have_source=1
      continue
    fi
    key="$(trim "$(printf '%s' "$line" | cut -d= -f1)")"
    value="$(trim "$(printf '%s' "$line" | cut -d= -f2-)")"
    case "$key" in
      name)         name="$value" ;;
      repo)         repo="$value" ;;
      rev)          rev="$value" ;;
      license)      license="$value" ;;
      license_file) license_file="$value" ;;
      include)      includes+=("$value") ;;
      max_files)    max_files="$value" ;;
      *) die "$manifest: unknown key '$key'" ;;
    esac
  done < "$manifest"

  flush
}

main() {
  local wanted="${1:-}"
  local manifest found=0
  for manifest in "$SOURCES_DIR"/*.sources; do
    [[ -e "$manifest" ]] || die "no manifests in $SOURCES_DIR"
    if [[ -n "$wanted" && "$(basename "$manifest" .sources)" != "$wanted" ]]; then
      continue
    fi
    found=1
    fetch_category "$manifest"
  done
  (( found )) || die "no category named '$wanted'"

  if [[ "$DRY_RUN" == "1" ]]; then
    printf 'dry run: nothing written\n' >&2
  else
    printf 'done. Fetched content is gitignored and never committed.\n' >&2
  fi
}

main "$@"
