#!/usr/bin/env bash
# Build the size-gate samples with the `size` profile and report each
# binary's size relative to the empty baseline.
#
# Usage: benches/size.sh                 # kanna variants
#        benches/size.sh --check         # kanna variants, fail over budget
#        benches/size.sh --compare       # plus clap/lexopt/pico-args/argh/bpaf
#        benches/size.sh core decl:help  # selected "bin[:features]" variants
set -euo pipefail
cd "$(dirname "$0")/.."

check=0
compare=0
variants=()
for a in "$@"; do
  case "$a" in
    --check) check=1 ;;
    --compare) compare=1 ;;
    *) variants+=("$a") ;;
  esac
done
if [ ${#variants[@]} -eq 0 ]; then
  variants=(core "decl:" "decl:help" "decl:help,suggest" "decl:full" "macro:help" "macro:full")
  if [ $compare = 1 ]; then
    variants+=("lexopt:lexopt" "pico_args:pico-args" "argh:argh" "bpaf:bpaf" "clap:clap")
  fi
fi

# Budgets in KiB (SPEC.md §5, help raised by ADR-0018): core ≤ 20, help ≤ 75. "full" must stay under
# half of clap; that ratio is checked by --compare --check.
budget() {
  case "$1" in
    core) echo 20 ;;
    "decl:help") echo 75 ;;
    *) echo "" ;;
  esac
}

build() { # bin features
  if [ -n "$2" ]; then
    cargo build -q --profile size -p kanna-benches --bin "$1" --features "$2"
  else
    cargo build -q --profile size -p kanna-benches --bin "$1"
  fi
}

build empty ""
base=$(stat -c %s target/size/empty)
printf '%-24s %10s %12s %8s\n' variant bytes 'delta(KiB)' budget
printf '%-24s %10d %12.1f\n' empty "$base" 0
fail=0
clap_delta=""
full_delta=""
for v in "${variants[@]}"; do
  bin=${v%%:*}
  feat=""
  [[ "$v" == *:* ]] && feat=${v#*:}
  build "$bin" "$feat"
  s=$(stat -c %s "target/size/$bin")
  delta=$(echo "($s - $base)/1024" | bc -l)
  label="$bin"
  [ -n "$feat" ] && label="$bin[$feat]"
  [[ "$v" == *:* && -z "$feat" ]] && label="$bin[no features]"
  b=$(budget "$v")
  printf '%-24s %10d %12.1f %8s\n' "$label" "$s" "$delta" "${b:+≤ $b}"
  if [ -n "$b" ] && [ "$(echo "$delta > $b" | bc -l)" = 1 ]; then
    echo "  !! $label exceeds its budget of $b KiB" >&2
    fail=1
  fi
  [ "$v" = "clap:clap" ] && clap_delta=$delta
  [ "$v" = "decl:full" ] && full_delta=$delta
done
if [ -n "$clap_delta" ] && [ -n "$full_delta" ]; then
  ratio=$(echo "$full_delta / $clap_delta" | bc -l)
  printf 'decl[full] / clap = %.2f (must be < 0.5)\n' "$ratio"
  if [ "$(echo "$ratio >= 0.5" | bc -l)" = 1 ]; then
    echo "  !! decl[full] is not under half of clap" >&2
    fail=1
  fi
fi
[ $check = 1 ] && exit $fail
exit 0
