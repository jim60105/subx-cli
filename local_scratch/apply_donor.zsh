#!/bin/zsh
# Hand-apply donor deltas of split-mixed-capabilities-across-repos into openspec/specs/.
# SUBSTITUTE not append. Exact-string title matching. Count + dedup guarded.
# Run from worktree root. NO python (series convention).
set -e
cd /var/home/jim60105/repos/subx-cli/.worktrees/split-mixed-capabilities-across-repos
CH=openspec/changes/split-mixed-capabilities-across-repos/specs
SPECS=${DRYSPECS:-openspec/specs}

# titles under a given "## <OP> Requirements" section
titles() { # $1 file $2 OP
  awk -v OP="$2" '$0 == "## " OP " Requirements" {f=1; next} f && /^## / {f=0} f && /^### Requirement: / {sub(/^### Requirement: /,""); print}' "$1"
}
# blocks of a section, US (\037) separated; each block starts at its header
blocks() { # $1 file $2 OP
  awk -v OP="$2" '
    $0 == "## " OP " Requirements" {inb=1; next}
    inb && /^## / {inb=0}
    inb {
      if ($0 ~ /^### Requirement: /) { if (have) printf "%c\n", 31; have=1 }
      if (have) print
    }
    END { if (have) printf "%c\n", 31 }
  ' "$1"
}

ndup() { # file -> duplicate title count
  sed -n 's/^### Requirement: //p' "$1" | sort | uniq -d | wc -l | tr -d ' '
}

for cap in cache-management component-factory configuration-management encoding-detection error-handling format-conversion input-path-handling parallel-processing secrets-protection subtitle-matching subtitle-translation supply-chain-hardening timeline-sync; do
  D=$CH/$cap/spec.md; S=$SPECS/$cap/spec.md
  rc=$(titles "$D" REMOVED | wc -l | tr -d ' '); mc=$(titles "$D" MODIFIED | wc -l | tr -d ' '); ac=$(titles "$D" ADDED | wc -l | tr -d ' ')
  # guard: no title may appear in two sections of the same delta
  for a in REMOVED MODIFIED; do for b in MODIFIED ADDED; do
    o=$(comm -12 <(titles "$D" $a | sort) <(titles "$D" $b | sort) | wc -l | tr -d ' ')
    [ "$o" = 0 ] || { echo "GUARD1 $cap: '$a'/'$b' overlap"; titles "$D" $a | sort > /tmp/a1; titles "$D" $b | sort > /tmp/b1; comm -12 /tmp/a1 /tmp/b1; exit 1; }
  done; done
  before=$(sed -n 's/^### Requirement: //p' "$S" | wc -l | tr -d ' ')
  # every REMOVED title must currently exist exactly once in the live spec
  while IFS= read -r t; do
    [ -n "$t" ] || continue
    got=$(sed -n 's/^### Requirement: //p' "$S" | awk -v t="$t" '$0==t{c++} END{print c+0}')
    [ "$got" = 1 ] || { echo "GUARD1b $cap: REMOVED '$t' present $got times in live spec"; exit 1; }
  done < <(titles "$D" REMOVED)
  # every MODIFIED title must currently exist exactly once (it stays, rewritten)
  while IFS= read -r t; do
    [ -n "$t" ] || continue
    got=$(sed -n 's/^### Requirement: //p' "$S" | awk -v t="$t" '$0==t{c++} END{print c+0}')
    [ "$got" = 1 ] || { echo "GUARD1c $cap: MODIFIED '$t' present $got times in live spec"; exit 1; }
  done < <(titles "$D" MODIFIED)
  echo "$cap: before=$before R=$rc M=$mc A=$ac"

  # 1) REMOVED: delete each exact-titled block
  while IFS= read -r t; do
    [ -n "$t" ] || continue
    awk -v T="### Requirement: $t" '
      { if (skip) { if ($0 ~ /^### Requirement: / || $0 ~ /^## /) skip=0; else next }
        if (!hit && $0 == T) { hit=1; skip=1; next }
        print }
    ' "$S" > "$S.tmp"; mv "$S.tmp" "$S"
    got=$(sed -n 's/^### Requirement: //p' "$S" | awk -v t="$t" '$0==t{c++} END{print c+0}')
    [ "$got" = 0 ] || { echo "DEL FAIL $cap: '$t' still present ($got)"; exit 1; }
  done < <(titles "$D" REMOVED)
  after_del=$(sed -n 's/^### Requirement: //p' "$S" | wc -l | tr -d ' ')
  [ "$after_del" = "$((before - rc))" ] || { echo "GUARD2 $cap: $before-$rc != $after_del"; exit 1; }

  # 2) MODIFIED: substitute each exact-titled block in place
  blocks "$D" MODIFIED > /tmp/mod.$cap
  nrec=0
  while IFS= read -r -d $'\037' blk; do
    [ -n "$blk" ] || continue
    nrec=$((nrec+1))
    t=$(printf '%s' "$blk" | sed -n '1s/^### Requirement: //p')
    printf '%s' "$blk" > /tmp/oneblk
    awk -v T="### Requirement: $t" -v REPL=/tmp/oneblk '
      BEGIN { while ((getline l < REPL) > 0) r[++n] = l }
      { if (skip) { if ($0 ~ /^### Requirement: / || $0 ~ /^## /) skip=0; else next }
        if (!done && $0 == T) { done=1; skip=1; for (i=1;i<=n;i++) print r[i]; next }
        print }
      END { exit done ? 0 : 1 }
    ' "$S" > "$S.tmp" || { echo "MOD MISS $cap: '$t' header not found"; exit 1; }
    mv "$S.tmp" "$S"
    got=$(sed -n 's/^### Requirement: //p' "$S" | awk -v t="$t" '$0==t{c++} END{print c+0}')
    [ "$got" = 1 ] || { echo "MOD FAIL $cap: '$t' count=$got"; exit 1; }
  done < /tmp/mod.$cap
  [ "$nrec" = "$mc" ] || { echo "GUARD3 $cap: substituted $nrec of $mc"; exit 1; }

  # 3) ADDED: append at end, blank-line separated
  if [ "$ac" -gt 0 ]; then
    blocks "$D" ADDED > /tmp/add.$cap
    : > /tmp/app.txt
    while IFS= read -r -d $'\037' blk; do
      [ -n "$blk" ] || continue
      printf '\n%s' "${blk%"${blk##*[![:space:]]}"}" >> /tmp/app.txt
    done < /tmp/add.$cap
    # trim trailing whitespace of file, append blocks, single trailing newline
    printf '%s\n' "$(cat "$S")" > "$S.tmp"
    cat /tmp/app.txt >> "$S.tmp"
    mv "$S.tmp" "$S"
  fi

  after=$(sed -n 's/^### Requirement: //p' "$S" | wc -l | tr -d ' ')
  exp=$((before - rc + ac))
  [ "$after" = "$exp" ] || { echo "GUARD4 $cap: $after != $exp"; exit 1; }
  d=$(ndup "$S")
  [ "$d" = 0 ] || { echo "DEDUP FAIL $cap: $d duplicate titles"; sed -n 's/^### Requirement: //p' "$S" | sort | uniq -d; exit 1; }
  echo "$cap: -> after=$after dup=$d OK"
done
echo ALL-DONE
