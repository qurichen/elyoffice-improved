#!/usr/bin/env bash
# Builds the demo notes repository at $1 with history.
set -euo pipefail

repo=${1:?repo path}
rm -rf "$repo"
mkdir -p "$repo"
cd "$repo"
git init -q -b main
git config user.name "Maya Lind"
git config user.email "maya@coastaltrails.example"
git config commit.gpgsign false

day=0
# Commits all changes as $1 with message $2.
commit() {
  day=$((day + 1))
  local when="2026-09-$(printf %02d "$day")T$((9 + day % 8)):$((10 + day * 7 % 50)):00"
  git add -A
  GIT_AUTHOR_NAME="$1" GIT_COMMITTER_NAME="$1" GIT_AUTHOR_EMAIL="$(tr 'A-Z ' 'a-z.' <<<"$1")@coastaltrails.example" \
    GIT_COMMITTER_EMAIL="$(tr 'A-Z ' 'a-z.' <<<"$1")@coastaltrails.example" \
    GIT_AUTHOR_DATE="$when" GIT_COMMITTER_DATE="$when" git commit -q -m "$2"
}

mkdir -p routes maps
cat >README.md <<'MD'
# Field notes

Working notes for the Coastal Trails field guide: one file per route,
plus the shared checklist and the map key.
MD
commit "Maya Lind" "Start the field notes"

cat >checklist.md <<'MD'
# Survey checklist

- Walk the route twice, once each direction
- Record timings at every junction
- Photograph each waymark
MD
commit "Maya Lind" "Add the survey checklist"

cat >routes/harbor-loop.md <<'MD'
# Harbor Loop

Distance: 4.1 km
Time: 1 h 05 min
Grade: Easy

## Directions

Start at the harbor car park.
Follow the sea wall east past the fish market.
At the slipway, keep left of the boat sheds.
Climb the steps to the lighthouse.
Take the seaward path round the lamp room.
Drop down to the beach by the coastguard hut.
Return through the dunes behind the beach.

## Notes

The dune path floods after heavy rain.
MD
commit "Maya Lind" "Survey the Harbor Loop"

cat >routes/cliff-path-north.md <<'MD'
# Cliff Path North

Distance: 9.6 km
Time: 3 h 10 min
Grade: Moderate

From Gull Point the path climbs onto the headland.
Two steep descents into coves with stepping stones.
MD
commit "Theo Brandt" "Survey Cliff Path North"

printf '\n- Note the tide at both coves\n' >>checklist.md
commit "Maya Lind" "Checklist: record tides at coves"

git switch -q -c maps
cat >maps/key.md <<'MD'
# Map key

- Dashed green: the route
- Solid dot: start and finish
- Blue wave: tidal crossing
MD
commit "Theo Brandt" "Draft the map key"
printf -- '- Triangle: viewpoint\n' >>maps/key.md
commit "Theo Brandt" "Map key: add viewpoints"
printf -- '- Cup: café or kiosk\n' >>maps/key.md
commit "Theo Brandt" "Map key: add refreshments"

git switch -q main
cat >routes/dune-meadows.md <<'MD'
# Dune Meadows

Distance: 5.3 km
Time: 1 h 30 min
Grade: Easy

Boardwalks through the reserve. Ponies graze the meadows.
MD
commit "Ines Okafor" "Survey Dune Meadows"
sed -i '' 's/Two steep descents/Two steep, short descents/' routes/cliff-path-north.md
commit "Ines Okafor" "Copy edit Cliff Path North"
day=$((day + 1))
GIT_AUTHOR_DATE="2026-09-$(printf %02d "$day")T16:20:00" GIT_COMMITTER_DATE="2026-09-$(printf %02d "$day")T16:20:00" \
  git merge -q --no-ff maps -m "Merge the map key"

git switch -q -c tide-tables
cat >tides.md <<'MD'
# Tide tables

| Harbor | Spring low | Neap low |
| --- | --- | --- |
| Gull Point | 06:40 | 09:15 |
| Lantern Bay | 07:05 | 09:40 |
MD
commit "Sam Reyes" "Start the tide tables"
printf '| Old Harbor | 06:55 | 09:30 |\n' >>tides.md
commit "Sam Reyes" "Tides: add Old Harbor"

git switch -q main
cat >routes/lantern-bay.md <<'MD'
# Lighthouse to Lantern Bay

Distance: 7.2 km
Time: 2 h 20 min
Grade: Moderate

Linear walk between two lighthouses. Coastguard steps at the north end.
MD
commit "Maya Lind" "Survey Lighthouse to Lantern Bay"
sed -i '' 's/Ponies graze the meadows./Ponies graze the meadows. Orchids in early summer./' routes/dune-meadows.md
commit "Ines Okafor" "Dune Meadows: note the orchids"

# Unstaged work for the recording: two hunks in one file.
cat >routes/harbor-loop.md <<'MD'
# Harbor Loop

Distance: 4.1 km
Time: 1 h 05 min
Grade: Easy
Best at: low tide

## Directions

Start at the harbor car park.
Follow the sea wall east past the fish market.
At the slipway, keep left of the boat sheds.
Climb the steps to the lighthouse.
Take the seaward path round the lamp room.
Drop down to the beach by the coastguard hut.
Return through the dunes behind the beach.

## Notes

The dune path floods after heavy rain; take the road above it instead.
Café at the harbor, open daily from eight.
MD
