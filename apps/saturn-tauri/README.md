# Easy Edit Pro · Tauri app

This Tauri 2 desktop target uses React, TypeScript, and Vite for the editor UI. Its Rust command bridge reuses the GTK-independent `saturn-core`; the existing GTK app remains available during migration.

## Development

Install the Tauri 2 Linux system dependencies, including WebKitGTK 4.1, Rust, Node.js, and pnpm. See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

From this directory:

```sh
pnpm install
pnpm tauri dev
```

Build the frontend assets and type-check them with:

```sh
pnpm build
```

The UI supports New/Open/Save project workflows, native multi-file import, GStreamer metadata discovery, a searchable media bin, source and program previews, timeline clip add/move/remove, fullscreen and close, and Rust-backed undo/redo. Color controls, per-track gain, and clip keyframes persist in schema-versioned project files and affect the webview program preview. Native rendering/export, full audio mixing, asset-provider APIs, AI features, and Linux release packages remain future work.

Run Rust core workflow tests and build the Tauri UI with:

```sh
cargo test -p saturn-core
pnpm build
```
