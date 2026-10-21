#!/usr/bin/env bash
# Pulls stills at boundaries and fixes, plus contact sheets.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
film="$here/../out/ElyOffice-launch.mp4"
out="$here/../out/edit/check"
rm -rf "$out"
mkdir -p "$out"
ffprobe -v error -show_entries format=duration:stream=codec_name,profile,width,height,r_frame_rate,pix_fmt -of compact "$film"
frames=(0 60 128 129 250 386 450 513 514 515 540 560 570 576 578 579 640 700 771 772 800 830 835 836 900 957 965 1011
  1029 1080 1092 1093 1150 1286 1330 1349 1350 1427 1430 1500 1543 1580 1600 1607 1608 1700 1800 1850 1864 1865 1900 1990
  2057 2185 2186 2200 2230 2260 2290 2310 2314 2315 2350 2400 2420 2470 2530 2572 2620 2699)
expr=$(printf "+eq(n\\,%s)" "${frames[@]}")
ffmpeg -v error -y -i "$film" -vf "select='${expr:1}'" -fps_mode passthrough -frame_pts 1 "$out/%05d.png"
ls "$out"/*.png | wc -l
cd "$out"
ls [0-9]*.png | split -l 12 - sheet-
for list in sheet-*; do
  args=()
  while read -r f; do args+=(-i "$f"); done < "$list"
  n=$(wc -l < "$list" | tr -d ' ')
  ffmpeg -v error -y "${args[@]}" -filter_complex "$(for i in $(seq 0 $((n - 1))); do printf "[%d]scale=640:360[s%d];" "$i" "$i"; done)$(for i in $(seq 0 $((n - 1))); do printf "[s%d]" "$i"; done)xstack=inputs=$n:layout=$(python3 -c "print('|'.join(f'{(i%3)*640}_{(i//3)*360}' for i in range($n)))"):fill=black" "$list.png"
  rm "$list"
done
