#!/usr/bin/env bash
# ElyShow: add animations, preview them, then present.
source "$(dirname "$0")/lib.sh"

us_input
launch ely-show "$demo/Coastal Trails 2027.pptx"
wait_log "lok: first tile drawn" 60
front
sleep 4
x=$win_x
y=$win_y
# Picks effect $1 rows above the Add Animation button.
animate() {
  local ax ay
  read -r ax ay < <(where "title=Add Animation")
  mouse click "$ax" "$ay"
  pause 0.6
  mouse click $((ax - 19)) $((ay - $1))
}
record_start show
pause 0.8
mark "select the title"
mouse click $((x + 750)) $((y + 352))
pause 0.6
mouse move $((x + 1440)) $((y + 600)) 0.5
mouse scroll -900 0.8
pause 0.4
mark "Add Animation > Fly In"
animate 359
pause 0.8
mark "select the subtitle"
mouse click $((x + 750)) $((y + 430))
pause 0.6
mark "Add Animation > Fade In"
animate 387
pause 0.8
read -r px py < <(where "title=Preview Animations")
mark "Preview Animations"
mouse click "$px" "$py"
pause 4.0
record_stop show

# The slide show fills the screen; record its middle.
mouse move 250 1100 0.4
keys "key code 96"
sleep 3
record_start show-play 491 30 2027 1140
pause 0.8
mark "Space: title flies in"
keys "key code 49"
pause 1.6
mark "Space: subtitle fades in"
keys "key code 49"
pause 1.6
mark "Right: slide 2"
keys "key code 124"
pause 2.0
mark "Right: slide 3"
keys "key code 124"
pause 2.0
record_stop show-play
keys "key code 53"
pause 1
