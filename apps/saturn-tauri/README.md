# Project Saturn · Tauri app

This Tauri 2 desktop target uses React, TypeScript, and Vite for the editor UI. Its Rust command bridge reuses the GTK-independent `saturn-core`; the existing GTK app remains available while Saturn's media and timeline workflows are migrated.

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

The current UI loads project details from `saturn-core`, supports workspace switching and media search, and includes fullscreen and close controls. Media import, playback, timeline editing, rendering, and package bundling still need migration work.
