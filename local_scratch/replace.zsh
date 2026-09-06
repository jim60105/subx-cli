#!/bin/zsh
# replace.zsh FILE OLD NEW  — literal substring replace; MUST match exactly once.
set -u
f=$1; old=$2; new=$3
content=$(< "$f")
count=0
rest=$content
while true; do
  idx=${rest#*$old}
  if [ "$idx" = "$rest" ]; then break; fi
  count=$((count+1))
  rest=$idx
done
if [ "$count" -ne 1 ]; then
  echo "FAIL($count): [$old]" ; exit 1
fi
out=${content//"$old"/"$new"}
printf '%s\n' "$out" > "$f"
echo "OK: replaced in $f"
