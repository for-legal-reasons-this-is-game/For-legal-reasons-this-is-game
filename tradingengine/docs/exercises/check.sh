#!/usr/bin/env bash
# Runs the exercises in order and stops at the first one that fails.
#
#   ./check.sh               every exercise, in order
#   ./check.sh ex05          only the exercises whose name starts with ex05
#   ./check.sh --solutions   the reference solutions (should all pass)
#
# proto needs protoc, so run it inside the engine's dev shell:
#   cd tradingengine && nix develop --command docs/exercises/check.sh
set -euo pipefail
cd "$(dirname "$0")"

prefix="ex"
filter="${1:-}"
if [[ "$filter" == "--solutions" ]]; then
    prefix="sol"
    filter=""
fi

for file in solutions/ex*.rs; do
    name="$(basename "$file" .rs)"           # ex03_validate_place
    target="${prefix}${name#ex}"             # ex03_... or sol03_...
    [[ -n "$filter" && "$name" != "$filter"* ]] && continue

    if cargo test --quiet --test "$target" >/tmp/engine_sprint_$$.log 2>&1; then
        echo "  done  $target"
    else
        cat /tmp/engine_sprint_$$.log
        rm -f /tmp/engine_sprint_$$.log
        echo
        echo "  todo  $target    file: exercises/$name.rs    hint: HINTS.md#$name"
        exit 1
    fi
done
rm -f /tmp/engine_sprint_$$.log
echo "all done"
