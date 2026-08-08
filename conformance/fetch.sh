#!/usr/bin/env bash
#
# Fetch the official conformance suites at pinned commits.
#
# MASTER_PLAN §3.3: "Conformance adapters: yaml-test-suite, JSONTestSuite,
#                    toml-test ...; published pass-rate badges."
# MASTER_PLAN §4.1 P4: pass rates published "including honest failure lists".
#
# Nothing fetched here is committed. conformance/.gitignore enforces that.
#
# ## Why this is not corpora/fetch.sh
#
# The corpus is a *sample*: 1,000 files chosen to exercise K1, capped, drawn
# from ten sources that all have the same shape. A conformance suite is the
# opposite on every axis — it is taken *complete*, it is not capped, there are
# exactly two of them, and each carries an accept/reject expectation the corpus
# has no equivalent of. yaml-test-suite additionally needs two pinned refs,
# because its generated `data` branch carries no licence file.
#
# Two sources with two different shapes do not justify a manifest language.
# The pins are right here, and `verify` below asserts the exact case counts, so
# a silently-changed suite fails loudly rather than quietly moving a pass rate.

set -euo pipefail
export LC_ALL=C

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DRY_RUN="${DRY_RUN:-0}"

# --- pins -------------------------------------------------------------------
# Full 40-char commit SHAs. Never a branch or a tag: those move, and a moving
# conformance suite turns a published pass rate into a number about whatever
# upstream looked like that morning.

JSON_REPO="https://github.com/nst/JSONTestSuite.git"
JSON_REV="1ef36fa01286573e846ac449e8683f8833c5b26a"
JSON_LICENSE="LICENSE"          # MIT
JSON_EXPECT_TOTAL=318           # test_parsing/*.json
JSON_EXPECT_ACCEPT=95           # y_*  must parse
JSON_EXPECT_REJECT=188          # n_*  must be rejected
JSON_EXPECT_EITHER=35           # i_*  implementation-defined, scored separately

YAML_REPO="https://github.com/yaml/yaml-test-suite.git"
# The `data` branch holds the extracted form: one directory per case with
# in.yaml and an `error` marker. The src/ form on main stores each case *inside
# a YAML document*, which would require a YAML parser to read the suite that
# tests our YAML parser.
YAML_DATA_REV="6ad3d2c62885d82fc349026c136ef560838fdf3d"
# The data branch is generated and carries no licence file, so the licence is
# taken from a pinned commit of the main branch. Recording it from somewhere is
# not optional — §10 requires upstream licences be respected, and "the branch we
# used had no LICENSE" is not a licence.
YAML_MAIN_REV="da267a5c4782e7361e82889e76c0dc7df0e1e870"
YAML_LICENSE="License"          # MIT
YAML_EXPECT_TOTAL=402           # in.yaml files, including nested <ID>/<NN>/
YAML_EXPECT_REJECT=94           # in.yaml with an `error` marker alongside
YAML_EXPECT_ACCEPT=308

die() { printf 'conformance/fetch.sh: %s\n' "$*" >&2; exit 1; }
note() { printf '  %s\n' "$*" >&2; }

command -v git >/dev/null || die "git is required"
SHA_CMD="sha256sum"; command -v sha256sum >/dev/null || SHA_CMD="shasum -a 256"
sha_of() { $SHA_CMD "$1" | awk '{print $1}'; }

WORKDIRS=()
cleanup() { local d; for d in ${WORKDIRS+"${WORKDIRS[@]}"}; do rm -rf "$d"; done; }
trap cleanup EXIT

# clone_at <repo> <rev> -> prints the checkout path
clone_at() {
  local repo="$1" rev="$2"
  [[ "$rev" =~ ^[0-9a-f]{40}$ ]] || die "rev '$rev' is not a 40-char commit SHA"
  local work; work="$(mktemp -d)"; WORKDIRS+=("$work")
  git -C "$work" init --quiet
  git -C "$work" remote add origin "$repo"
  git -C "$work" fetch --quiet --depth 1 origin "$rev" \
    || die "cannot fetch $rev from $repo (force-pushed away?)"
  git -C "$work" checkout --quiet FETCH_HEAD
  printf '%s' "$work"
}

fetch_json() {
  note "JSONTestSuite  <-  ${JSON_REV:0:12}  (MIT)"
  if [[ "$DRY_RUN" == "1" ]]; then return 0; fi
  local work; work="$(clone_at "$JSON_REPO" "$JSON_REV")"
  [[ -f "$work/$JSON_LICENSE" ]] || die "JSONTestSuite: $JSON_LICENSE missing at $JSON_REV"

  local dest="$HERE/json-test-suite"
  rm -rf "$dest"; mkdir -p "$dest"
  cp "$work/$JSON_LICENSE" "$dest/UPSTREAM-LICENSE"
  cp -r "$work/test_parsing" "$dest/test_parsing"

  {
    printf '#! suite = json-test-suite\n'
    printf '#! repo = %s\n' "$JSON_REPO"
    printf '#! rev = %s\n' "$JSON_REV"
    printf '#! license = MIT\n'
    printf '#! upstream_license_sha256 = %s\n' "$(sha_of "$dest/UPSTREAM-LICENSE")"
  } > "$dest/MANIFEST.tsv"
  (cd "$dest/test_parsing" && find . -name '*.json' | sort) \
    | while IFS= read -r rel; do
        printf '%s\t%s\n' "$(sha_of "$dest/test_parsing/$rel")" "${rel#./}"
      done >> "$dest/MANIFEST.tsv"

  local total accept reject either
  total=$(find "$dest/test_parsing" -name '*.json' | wc -l)
  accept=$(find "$dest/test_parsing" -name 'y_*.json' | wc -l)
  reject=$(find "$dest/test_parsing" -name 'n_*.json' | wc -l)
  either=$(find "$dest/test_parsing" -name 'i_*.json' | wc -l)
  note "    $total cases: $accept must-accept, $reject must-reject, $either implementation-defined"
  (( total == JSON_EXPECT_TOTAL )) || die "expected $JSON_EXPECT_TOTAL cases, got $total"
  (( accept == JSON_EXPECT_ACCEPT )) || die "expected $JSON_EXPECT_ACCEPT y_ cases, got $accept"
  (( reject == JSON_EXPECT_REJECT )) || die "expected $JSON_EXPECT_REJECT n_ cases, got $reject"
  (( either == JSON_EXPECT_EITHER )) || die "expected $JSON_EXPECT_EITHER i_ cases, got $either"
}

fetch_yaml() {
  note "yaml-test-suite  <-  data@${YAML_DATA_REV:0:12}  (licence from main@${YAML_MAIN_REV:0:12}, MIT)"
  if [[ "$DRY_RUN" == "1" ]]; then return 0; fi
  local data; data="$(clone_at "$YAML_REPO" "$YAML_DATA_REV")"
  local main; main="$(clone_at "$YAML_REPO" "$YAML_MAIN_REV")"
  [[ -f "$main/$YAML_LICENSE" ]] || die "yaml-test-suite: $YAML_LICENSE missing at $YAML_MAIN_REV"

  local dest="$HERE/yaml-test-suite"
  rm -rf "$dest"; mkdir -p "$dest/cases"
  cp "$main/$YAML_LICENSE" "$dest/UPSTREAM-LICENSE"

  # One directory per case: in.yaml, plus an `error` marker when the case is
  # expected to be rejected. Nested <ID>/<NN>/ dirs are multi-document cases and
  # are kept as distinct cases.
  (cd "$data" && find . -name in.yaml -not -path './.git/*' | sort) \
    | while IFS= read -r rel; do
        case_dir="$(dirname "${rel#./}")"
        mkdir -p "$dest/cases/$case_dir"
        cp "$data/$case_dir/in.yaml" "$dest/cases/$case_dir/in.yaml"
        # Explicit `if`, not `[[ ... ]] && cmd`: as the last statement of a loop
        # body the latter returns 1 when the file is absent, which is a latent
        # `set -e` abort on the 308 cases that have no error marker.
        if [[ -f "$data/$case_dir/error" ]]; then
          : > "$dest/cases/$case_dir/error"
        fi
        if [[ -f "$data/$case_dir/===" ]]; then
          cp "$data/$case_dir/===" "$dest/cases/$case_dir/name"
        fi
      done

  {
    printf '#! suite = yaml-test-suite\n'
    printf '#! repo = %s\n' "$YAML_REPO"
    printf '#! rev = %s\n' "$YAML_DATA_REV"
    printf '#! license_rev = %s\n' "$YAML_MAIN_REV"
    printf '#! license = MIT\n'
    printf '#! upstream_license_sha256 = %s\n' "$(sha_of "$dest/UPSTREAM-LICENSE")"
  } > "$dest/MANIFEST.tsv"
  (cd "$dest/cases" && find . -name in.yaml | sort) \
    | while IFS= read -r rel; do
        printf '%s\t%s\n' "$(sha_of "$dest/cases/${rel#./}")" "${rel#./}"
      done >> "$dest/MANIFEST.tsv"

  local total reject
  total=$(find "$dest/cases" -name in.yaml | wc -l)
  reject=$(find "$dest/cases" -name error | wc -l)
  note "    $total cases: $((total - reject)) must-accept, $reject must-reject"
  (( total == YAML_EXPECT_TOTAL )) || die "expected $YAML_EXPECT_TOTAL cases, got $total"
  (( reject == YAML_EXPECT_REJECT )) || die "expected $YAML_EXPECT_REJECT error cases, got $reject"
  (( total - reject == YAML_EXPECT_ACCEPT )) || die "must-accept count drifted"
}

main() {
  case "${1:-all}" in
    json) fetch_json ;;
    yaml) fetch_yaml ;;
    all)  fetch_json; fetch_yaml ;;
    *) die "usage: conformance/fetch.sh [json|yaml|all]" ;;
  esac
  if [[ "$DRY_RUN" == "1" ]]; then
    printf 'dry run: nothing written\n' >&2
  else
    printf 'done. Fetched suites are gitignored and never committed.\n' >&2
  fi
}

main "$@"
