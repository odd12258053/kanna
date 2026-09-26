#!/usr/bin/env bash
# Clean build times of the sample CLIs, dev and release profiles.
#
# Usage: benches/build-time.sh            # hasami variants
#        benches/build-time.sh --compare  # plus the competing libraries
#        benches/build-time.sh --check    # fail when over budget
#
# Budgets (SPEC.md §5): core ≤ 2 s, full features without derive ≤ 4 s.
# Budgets are for a clean build of the crate graph *excluding* the one-off
# download of dependencies, so `cargo fetch` runs first.
set -euo pipefail
cd "$(dirname "$0")/.."

check=0
compare=0
for a in "$@"; do
  case "$a" in
    --check) check=1 ;;
    --compare) compare=1 ;;
  esac
done
variants=(core "decl:help" "decl:full" "macro:full")
if [ $compare = 1 ]; then
  variants+=("lexopt:lexopt" "pico_args:pico-args" "argh:argh" "bpaf:bpaf" "clap:clap")
fi
budget() {
  case "$1" in
    core) echo 2 ;;
    "decl:full") echo 4 ;;
    *) echo "" ;;
  esac
}
cargo fetch -q
timed() { # profile bin features
  local flags=(-q --profile "$1" -p hasami-benches --bin "$2")
  [ -n "$3" ] && flags+=(--features "$3")
  cargo clean -q --profile "$1" 2>/dev/null || cargo clean -q
  local start end
  start=$(date +%s.%N)
  cargo build "${flags[@]}"
  end=$(date +%s.%N)
  echo "$end - $start" | bc -l
}
printf '%-20s %10s %10s %8s\n' variant 'dev (s)' 'release (s)' budget
fail=0
for v in "${variants[@]}"; do
  bin=${v%%:*}
  feat=""
  [[ "$v" == *:* ]] && feat=${v#*:}
  dev=$(timed dev "$bin" "$feat")
  rel=$(timed release "$bin" "$feat")
  b=$(budget "$v")
  label="$bin"; [ -n "$feat" ] && label="$bin[$feat]"
  printf '%-20s %10.2f %10.2f %8s\n' "$label" "$dev" "$rel" "${b:+≤ $b s}"
  if [ -n "$b" ] && [ "$(echo "$dev > $b" | bc -l)" = 1 ]; then
    echo "  !! $label dev build exceeds $b s" >&2
    fail=1
  fi
done
[ $check = 1 ] && exit $fail
exit 0
