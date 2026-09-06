#!/bin/zsh
# Extract the 21 donor parent blocks needed for the 23 composed core halves
# into /tmp/parents.txt (Tolerant Load Path parent Repair Path and Task
# Scheduler parents already read elsewhere).
set -e
cd /var/home/jim60105/repos/subx-cli/.worktrees/split-mixed-capabilities-across-repos
OUT=/tmp/parents.txt
: > $OUT
extract() { # $1 cap $2 title
  awk -v T="### Requirement: $2" -v OUT="$OUT" '
    BEGIN{on=0}
    on && (($0 ~ /^### Requirement: / && $0 != T) || /^## [^#]/) {exit}
    $0 == T {on=1}
    on {print >> OUT}
  ' openspec/specs/$1/spec.md
  printf '\n' >> $OUT
}
while IFS='|' read -r cap t; do
  printf '\n===== [%s] %s\n' "$cap" "$t" >> $OUT
  extract "$cap" "$t"
done <<'PAIRS'
encoding-detection|Robust Handling of Empty and Binary Files
error-handling|User-Facing Error Formatting
error-handling|No Panics On Recoverable Errors
error-handling|Library and Binary Error Surface Split
format-conversion|Supported Output Formats
secrets-protection|Mask sensitive config values in CLI output
input-path-handling|Core-Owned Input Collection
input-path-handling|Unified Path Merging
input-path-handling|Extension Filtering
input-path-handling|Recursive vs Flat Traversal
input-path-handling|No-Extract CLI Flag
input-path-handling|CollectedFiles Additional APIs
subtitle-matching|AI-Based File Pairing
subtitle-matching|Confidence Threshold Enforcement
subtitle-matching|File Relocation Modes
subtitle-matching|Optional Backup Before Move
subtitle-matching|AI-Driven Language and Globally-Unique Target Naming
subtitle-translation|Translation Guidance Options
timeline-sync|Sync Method Selection
timeline-sync|Core-Owned Sync Pairing Resolution
timeline-sync|Core-Owned Default Output Path Derivation
PAIRS
wc -l $OUT
