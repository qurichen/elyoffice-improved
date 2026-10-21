#!/usr/bin/env bash
# ElyMark: visual typing, formula and diagram, source and back.
source "$(dirname "$0")/lib.sh"

notes=/Users/Shared/Notes
mkdir -p "$notes"
cp "$demo/harbor-loop.md" "$notes/"
us_input
launch ely-mark "$notes/harbor-loop.md"
wait_log 'mark: shown as "visual"' 60
front
sleep 3
x=$win_x
y=$win_y
record_start mark
pause 0.8
mark "click end of the intro paragraph"
mouse click $((x + 1000)) $((y + 235))
pause 0.4
mark "typing"
typed " Allow two hours in winter."
pause 0.6
mark "scroll to formula and Mermaid diagram"
mouse move $((x + 1000)) $((y + 520)) 0.5
mouse scroll -1000 1.8
pause 1.8
mark "Cmd+2: source mode"
keys 'keystroke "2" using command down'
pause 2.2
mark "Cmd+1: back to visual"
keys 'keystroke "1" using command down'
pause 2.2
record_stop mark
quit_app
rm -rf "$notes"
