#!/bin/zsh
# apply_delta.sh SPECS_DIR DELTAS_DIR CAP
# REMOVED -> delete block; MODIFIED -> substitute in place; ADDED -> append under ## Requirements.
set -u
SPECS=$1; DELTAS=$2; CAP=$3
SPEC=$SPECS/$CAP/spec.md
DELTA=$DELTAS/$CAP/spec.md
TMP=$(mktemp -d); trap 'rm -rf $TMP' EXIT

titles() { awk -v op="$1" '
$0 == "## " op " Requirements" { f=1; next }
/^## / { if ($0 != "## " op " Requirements") f=0 }
f && /^### Requirement: / { sub(/^### Requirement: /,""); print }' "$DELTA"; }

# extract block of title from stdin file $1, strip Reason/Migration paragraphs
extract_blk() {
  awk -v title="$2" '
    BEGIN { inblk=0 }
    { if (inblk && ($0 ~ /^### / || $0 ~ /^## /)) exit
      if (!inblk && $0 == "### Requirement: " title) { inblk=1; print; next }
      if (inblk) print }' "$1" > "$TMP/raw.md"
  awk 'BEGIN{d=0}
    { if ($0 ~ /^\*\*Reason\*\*:/ || $0 ~ /^\*\*Migration\*\*:/) { d=1; next }
      if (d==1 && $0=="") { d=0; next }
      d=0; buf[++n]=$0 }
    END { while (n>0 && buf[n]=="") n--; for (i=1;i<=n;i++) print buf[i] }' "$TMP/raw.md" > "$TMP/blk.md"
}

nr=$(titles REMOVED | grep -c . || true)
nm=$(titles MODIFIED | grep -c . || true)
na=$(titles ADDED | grep -c . || true)

cp "$SPEC" "$TMP/cur.md"

# --- deletions
while IFS= read -r t; do
  [ -z "$t" ] && continue
  grep -qxF "### Requirement: $t" "$TMP/cur.md" || { echo "MISSING-TO-DELETE($CAP): $t"; exit 1; }
  awk -v title="$t" '
    BEGIN { skip=0; hdr=0 }
    {
      if (!skip && $0 == "### Requirement: " title) { skip=1; hdr=1; next }
      if (skip) {
        if (!hdr && ($0 ~ /^### / || $0 ~ /^## /)) { skip=0; hdr=0; print; next }
        hdr=0; next
      }
      print
    }' "$TMP/cur.md" > "$TMP/next.md"
  mv "$TMP/next.md" "$TMP/cur.md"
done < <(titles REMOVED)

# --- substitutions
while IFS= read -r t; do
  [ -z "$t" ] && continue
  grep -qxF "### Requirement: $t" "$TMP/cur.md" || { echo "MISSING-TO-MODIFY($CAP): $t"; exit 1; }
  extract_blk "$DELTA" "$t"
  cp "$TMP/blk.md" "$TMP/new.md"
  awk -v title="$t" -v newf="$TMP/new.md" '
    BEGIN { skip=0; hdr=0 }
    {
      if (!skip && $0 == "### Requirement: " title) { skip=1; hdr=1
        while ((getline b < newf) > 0) print b; close(newf); print ""; next }
      if (skip) {
        if (!hdr && ($0 ~ /^### / || $0 ~ /^## /)) { skip=0; hdr=0; print; next }
        hdr=0; next
      }
      print
    }' "$TMP/cur.md" > "$TMP/next.md"
  mv "$TMP/next.md" "$TMP/cur.md"
done < <(titles MODIFIED)

# --- additions
while IFS= read -r t; do
  [ -z "$t" ] && continue
  grep -qxF "### Requirement: $t" "$TMP/cur.md" && { echo "ALREADY-PRESENT-TO-ADD($CAP): $t"; exit 1; }
  extract_blk "$DELTA" "$t"
  awk 'BEGIN{prev=""} { if (!($0=="" && prev=="")) print; prev=$0 }' "$TMP/cur.md" > "$TMP/next.md"
  mv "$TMP/next.md" "$TMP/cur.md"
  { printf '\n'; cat "$TMP/blk.md"; printf '\n'; } >> "$TMP/cur.md"
done < <(titles ADDED)

# collapse >1 consecutive blanks anywhere
awk 'BEGIN{c=0} { if ($0=="") { c++; if (c>1) next } else c=0; print }' "$TMP/cur.md" > "$SPEC"
echo "APPLIED $CAP: R=$nr M=$nm A=$na -> now $(sed -n 's/^### Requirement: //p' "$SPEC" | grep -c .) reqs"
