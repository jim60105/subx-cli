#!/bin/zsh
# deltitles.sh DELTA_FILE OP  -> print titles under "## OP Requirements"
awk -v op="$2" '
$0 == "## " op " Requirements" { flag=1; next }
/^## / && $0 != "## " op " Requirements" { flag=0 }
flag && /^### Requirement: / { sub(/^### Requirement: /,""); print }
' "$1"
