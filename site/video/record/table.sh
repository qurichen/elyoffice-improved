#!/usr/bin/env bash
# ElyTable: SUM row, sort by total, insert a chart.
source "$(dirname "$0")/lib.sh"

us_input
launch ely-table "$demo/Sales 2027.xlsx"
wait_log "lok: first tile drawn" 60
front
sleep 4
x=$win_x
y=$win_y
# Cell centers in window points.
cell() { echo $((x + $1)) $((y + $2)); }
menu() {
  osascript -e "tell application \"System Events\" to tell (first process whose unix id is $app_pid) to click menu item \"$2\" of menu \"$1\" of menu bar 1" >/dev/null
}
record_start table
pause 0.6
mark "click B9"
mouse click $(cell 242 357)
pause 0.2
mark "typing =SUM(B2:B7)"
typed "=SUM(B2:B7)"
pause 0.4
keys "key code 36"
mark "formula entered"
pause 0.3
mouse click $(cell 242 357)
pause 0.2
mark "fill handle drag B9 to H9"
mouse drag $(cell 295 370) $(cell 880 362) 0.8
pause 0.6
mark "select A1:H7"
mouse drag $(cell 118 145) $(cell 880 305) 0.6
pause 0.2
menu Data AutoFilter
mark "AutoFilter on"
pause 0.5
mouse click $(cell 921 147)
mark "Total filter menu open"
pause 0.6
read -r sx sy < <(where $'title=Sort Z to A\t')
mouse click "$sx" "$sy"
mark "sorted Z to A by Total"
pause 1.0
mouse click $(cell 1000 420)
pause 0.2
mark "select A1:G7"
mouse drag $(cell 118 145) $(cell 774 305) 0.6
pause 0.2
menu Insert "Chart…"
mark "Insert Chart dialog"
pause 0.5
read -r ix iy < <(where $'AXButton\tvalue=\ttitle=Insert\t')
mouse click $((ix - 87)) $((iy - 185))
typed "Sales by product"
pause 0.3
mouse click "$ix" "$iy"
mark "chart inserted"
pause 0.8
mouse drag $(cell 520 300) $(cell 520 610) 0.8
mark "chart moved below the table"
pause 0.4
mouse click $(cell 1050 520)
pause 1.5
record_stop table
