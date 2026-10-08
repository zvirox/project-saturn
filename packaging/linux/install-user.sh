#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
data_dir=${XDG_DATA_HOME:-"$HOME/.local/share"}
bin_dir=${SATURN_BIN_DIR:-"$HOME/.local/bin"}

cargo build --release --manifest-path "$repo_root/Cargo.toml" --bin project-saturn
install -Dm755 "$repo_root/target/release/project-saturn" "$bin_dir/project-saturn"
install -Dm644 "$repo_root/assets/icons/saturn-camera.png" \
  "$data_dir/icons/hicolor/256x256/apps/saturn-camera.png"
install -Dm644 "$repo_root/packaging/linux/com.zvirox.ProjectSaturn.desktop" \
  "$data_dir/applications/com.zvirox.ProjectSaturn.desktop"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$data_dir/icons/hicolor" >/dev/null 2>&1 || true
fi

printf 'Installed Project Saturn. Ensure %s is on PATH, then launch it from your app menu.\n' "$bin_dir"
