# Tauri migration direction

## Decision

Build a parallel Tauri desktop target while retaining the GTK app as a working fallback. The Tauri UI should call the existing Rust command engine rather than reimplementing editing state in JavaScript. Keep media discovery, timeline time, project serialization, and future rendering in Rust crates.

## Why Tauri

Tauri makes a polished, adaptive editor interface easier to build with web layout tools and uses the operating system's webview instead of shipping a separate browser runtime. This can reduce app bundle size compared with Electron, but it does not guarantee lower memory use or faster media playback than GTK. On Linux it depends on WebKitGTK 4.1, which is already available on the development machine.

## Migration stages

1. Keep `saturn-core` and `saturn-cli` framework-independent.
2. Prototype Tauri windowing, command IPC, workspace switching, and app packaging alongside GTK.
3. Validate a native media-preview path before replacing the GTK source monitor. Avoid sending uncompressed video frames through the UI command channel.
4. Port project creation/open/save and media import through the shared command IDs.
5. Migrate editing workspaces and playback/rendering as those Rust features land.
6. Make Tauri the default desktop target only after its UI, media path, packaging, and project workflows are usable; retire GTK after parity or a deliberate product decision.

## Current migration boundary

`apps/saturn-tauri` now uses React, TypeScript, and Vite. Its Rust command bridge calls `project.inspect` in `saturn-core`; the UI renders project details, workspace tabs, searchable media state, fullscreen, and close controls. Import, playback, timeline editing, rendering, and release packaging have not migrated yet. Keep GTK available until those workflows are implemented and verified in Tauri.
