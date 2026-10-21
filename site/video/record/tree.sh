#!/usr/bin/env bash
# ElyTree: diff, stage a hunk, commit, then the history graph.
source "$(dirname "$0")/lib.sh"

repo=/Users/Shared/Projects/field-notes
bash "$video/demo/repo.sh" "$repo"
us_input
PANEL_LEFT=540 EXTRA_ENV="ELY_GIT=/usr/bin/git" launch ely-tree "$repo"
wait_log "tree: watching" 30
front
sleep 3
x=$win_x
y=$win_y
record_start tree
pause 0.8
mark "open the changed file's diff"
mouse click $((x + 92)) $((y + 157))
pause 1.4
mark "Stage Hunk (first hunk only)"
mouse click $((x + 1544)) $((y + 150))
pause 1.0
mark "type commit summary"
mouse click $((x + 250)) $((y + 736))
pause 0.3
typed "Harbor Loop: best at low tide"
pause 0.5
mark "Commit to main"
mouse click $((x + 477)) $((y + 855))
pause 1.2
mark "History tab: graph"
mouse click $((x + 145)) $((y + 85))
pause 1.2
mark "open the new commit"
mouse click $((x + 260)) $((y + 156))
pause 1.0
mouse move $((x + 1000)) $((y + 700)) 0.8
pause 1.5
record_stop tree
rm -rf "$repo"
rmdir /Users/Shared/Projects 2>/dev/null || true
