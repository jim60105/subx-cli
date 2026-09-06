#!/bin/zsh
# assemble.zsh OUTFILE "DONORCAP|DONOR_TITLE[|NEW_TITLE]" ...
# Emits "## ADDED Requirements" then each block extracted from the donor main spec
# (renamed if NEW_TITLE given). One blank line between blocks.
set -u
OUT=$1; shift
ROOT=/var/home/jim60105/repos/subx-cli/.worktrees/split-mixed-capabilities-across-repos
echo "## ADDED Requirements" > "$OUT"
for spec in "$@"; do
  cap=${spec%%|*}; rest=${spec#*|}
  if [[ "$rest" == *"|"* ]]; then t=${rest%%|*}; nt=${rest#*|}; else t=$rest; nt=$rest; fi
  src=$ROOT/openspec/specs/$cap/spec.md
  grep -qxF "### Requirement: $t" "$src" || { echo "MISSING $t in $cap"; exit 1; }
  awk -v title="$t" '
    BEGIN { inblk=0 }
    { if (inblk && ($0 ~ /^### / || $0 ~ /^## /)) exit
      if (!inblk && $0 == "### Requirement: " title) { inblk=1 }
      if (inblk) print }' "$src" > "$ROOT/local_scratch/_blk.tmp"
  printf '\n' >> "$OUT"
  if [ "$nt" != "$t" ]; then
    sed "1s|^### Requirement: $t\$|### Requirement: $nt|" "$ROOT/local_scratch/_blk.tmp" >> "$OUT"
  else
    cat "$ROOT/local_scratch/_blk.tmp" >> "$OUT"
  fi
done
# trim: collapse >1 blank
awk 'BEGIN{c=0} { if ($0=="") { c++; if (c>1) next } else c=0; print }' "$OUT" > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"
echo "WROTE $OUT ($(sed -n 's/^### Requirement: //p' "$OUT" | grep -c .) reqs)"
