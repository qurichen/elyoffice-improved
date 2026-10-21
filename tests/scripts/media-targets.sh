#!/usr/bin/env bash
# The media player interface compiles for Linux and Windows too.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
targets=(x86_64-unknown-linux-gnu x86_64-pc-windows-msvc)
installed=$(cd "$root" && rustup target list --installed)
for target in "${targets[@]}"; do
  if ! grep -qx "$target" <<<"$installed"; then
    echo "SKIP: media-targets needs rustup target $target"
    exit 0
  fi
done
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/src"
cp "$root/rust-toolchain.toml" "$work/"
cat >"$work/Cargo.toml" <<TOML
[package]
name = "media-targets"
version = "0.1.0"
edition = "2024"
[dependencies]
anyhow = "1"
log = "0.4"
TOML
cat >"$work/src/lib.rs" <<RS
#[path = "$root/crates/ely-platform/src/media.rs"]
pub mod media;
// Every call ElyShow makes.
pub fn touch(path: &std::path::Path) -> anyhow::Result<Option<media::Frame>> {
    let player = media::Player::open(path)?;
    let _: media::State = player.state();
    let (_, _): (bool, f64) = (player.playing(), player.position());
    player.play();
    player.pause();
    player.seek(0.0);
    Ok(player.frame())
}
RS
for target in "${targets[@]}"; do
  (cd "$work" && cargo check --quiet --target "$target") \
    || { echo "FAIL: media does not compile for $target"; exit 1; }
done
echo "media-targets: the player interface compiles for ${targets[*]}"
