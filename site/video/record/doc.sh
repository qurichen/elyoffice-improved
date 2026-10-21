#!/usr/bin/env bash
# ElyDoc: type into the brief, bold a word, zoom in.
source "$(dirname "$0")/lib.sh"

us_input
launch ely-doc "$demo/Field Guide Brief.docx"
wait_log "lok: first tile drawn" 60
wait_log "doc: 3 headings"
front
sleep 3
read -r zx zy < <(where "title=zoom-in")
record_start doc
snap doc-before
# The page's left edge, past the outline column.
page=$(first_bright doc-before 600 252 225)
x=$((win_x + page))
y=$win_y
pause 0.6
mark "click end of Purpose paragraph"
mouse click $((x + 720)) $((y + 392))
pause 0.5
mark "typing starts"
typed " Every route also gets a printable offline map."
mark "typing ends"
pause 0.8
mark "double-click 'forty'"
mouse double $((x + 243)) $((y + 351))
pause 0.5
keys 'keystroke "b" using command down'
mark "bold applied"
pause 1.0
mouse click "$zx" "$zy"
mark "zoom to 125%"
pause 0.6
mouse move $((x + 760)) $((y + 640)) 0.7
pause 2.0
record_stop doc
