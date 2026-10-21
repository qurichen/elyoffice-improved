#!/usr/bin/env bash
# ElyPDF: scroll pages, highlight, draw and place a signature.
source "$(dirname "$0")/lib.sh"

us_input
launch ely-pdf "$demo/Coastal Trails Guide.pdf"
wait_log "pdf: first tile drawn" 60
front
sleep 4
x=$win_x
y=$win_y
menu() {
  osascript -e "tell application \"System Events\" to tell (first process whose unix id is $app_pid) to click menu item \"$2\" of menu \"$1\" of menu bar 1" >/dev/null
}
record_start pdf
pause 0.6
mark "scroll from page 1 to page 2"
mouse move $((x + 900)) $((y + 520)) 0.5
mouse scroll -2200 2.4
pause 0.4
menu View "Fit Page"
mark "Fit Page: whole page 2"
pause 0.8
menu Annotate Highlight
mark "highlight tool; drag over two lines"
mouse drag $((x + 688)) $((y + 522)) $((x + 1118)) $((y + 538)) 1.0
mark "highlight added"
pause 0.8
keys "key code 53"
menu Document "Add Visible Signature…"
mark "Visible signature dialog"
pause 0.6
mouse path 1.8 $(python3 "$video/record/signature.py" $((x + 770)) $((y + 495)) 280)
mark "signature drawn"
pause 0.5
read -r gx gy < <(where "title=Place on Page")
mouse click "$gx" "$gy"
pause 0.5
mouse click $((x + 960)) $((y + 710))
mark "signature placed on page"
pause 0.6
mouse move $((x + 1350)) $((y + 640)) 0.6
pause 1.4
record_stop pdf
