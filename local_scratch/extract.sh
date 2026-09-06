#!/bin/zsh
# extract.sh FILE "Requirement Title" [strip_notes]
# Print block from "### Requirement: <title>" up to (excl.) next ^### or ^## line.
# Trim trailing blanks. strip_notes=1 removes **Reason**/**Migration** paragraphs.
awk -v title="$2" -v strip="${3:-0}" '
BEGIN { inblk=0 }
{
  if (inblk && ($0 ~ /^### / || ($0 ~ /^## / && $0 !~ /^###/))) exit
  if (!inblk && $0 == "### Requirement: " title) inblk=1
  if (inblk) print
}' "$1" | awk -v strip="$3" '
BEGIN { drop=0 }
{
  if (strip=="1" && ($0 ~ /^\*\*Reason\*\*:/ || $0 ~ /^\*\*Migration\*\*:/)) { drop=1; next }
  if (drop==1 && $0=="") { drop=0; next }
  drop=0
  buf[++n]=$0
}
END { while (n>0 && buf[n]=="") n--; for (i=1;i<=n;i++) print buf[i] }'
