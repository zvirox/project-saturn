#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
data_dir=${XDG_DATA_HOME:-"$HOME/.local/share"}
bin_dir=${EASY_EDIT_PRO_BIN_DIR:-"$HOME/.local/bin"}
font_dir="$data_dir/fonts/Easy Edit Pro"

cargo build --release --manifest-path "$repo_root/Cargo.toml" --bin project-saturn
install -Dm755 "$repo_root/target/release/project-saturn" "$bin_dir/easy-edit-pro"
install -Dm644 "$repo_root/assets/icons/saturn-camera.png" \
  "$data_dir/icons/hicolor/256x256/apps/saturn-camera.png"
install -Dm644 "$repo_root/packaging/linux/easy-edit-pro.desktop" \
  "$data_dir/applications/easy-edit-pro.desktop"
install -Dm644 "$repo_root/assets/fonts/InterVariable.ttf" \
  "$font_dir/InterVariable.ttf"

if command -v fc-cache >/dev/null 2>&1; then
  fc-cache -f "$font_dir" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$data_dir/icons/hicolor" >/dev/null 2>&1 || true
fi

printf 'Installed Easy Edit Pro. Ensure %s is on PATH, then launch it from your app menu.\n' "$bin_dir"
