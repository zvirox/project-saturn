# Tauri migration direction

## Decision

Build a parallel Tauri desktop target while retaining the GTK app as a working fallback. The Tauri UI should call the existing Rust command engine rather than reimplementing editing state in JavaScript. Keep media discovery, timeline time, project serialization, and future rendering in Rust crates.

## Why Tauri

Tauri makes a polished, adaptive editor interface easier to build with web layout tools and uses the operating system's webview instead of shipping a separate browser runtime. This can reduce app bundle size compared with Electron, but it does not guarantee lower memory use or faster media playback than GTK. On Linux it depends on WebKitGTK 4.1, which is already available on the development machine.

## Migration stages

1. Keep `saturn-core` and `saturn-cli` framework-independent.
2. Prototype Tauri windowing, command IPC, workspace switching, and app packaging alongside GTK.
3. Validate a native media-preview path before replacing the GTK source monitor. Avoid sending uncompressed video frames through the UI command channel.
4. Port project creation/open/save and media import through the shared command IDs. (Implemented in the current Tauri target.)
5. Connect basic timeline operations, undo/redo, source/program preview, and GStreamer metadata discovery. (Initial implementation is in place; editing precision and playback behavior still need refinement.)
6. Implement persistent color/audio/keyframe controls. (Initial implementation now stores them in schema v3 and applies them to the Tauri preview.)
7. Add the native Rust rendering/export pipeline and complete audio mixing.
8. Add FOSS asset providers with attribution and define provider-backed AI features.
9. Add workflow, media compatibility, UI, and packaging verification. Make Tauri the default desktop target only after these are usable; retire GTK after parity or a deliberate product decision.

## Current migration boundary

`apps/saturn-tauri` uses React, TypeScript, and Vite. The Rust bridge calls shared `saturn-core` commands for project status, creation, open/save, media registration, timeline clip add/move/remove, and undo/redo. The native dialog plugin handles file selection; GStreamer discovers media metadata on a blocking worker; selected files are exposed through Tauri's scoped asset protocol for source and program previews. Timeline moves snap to project frame boundaries and reject incompatible audio/video track placements.

Project schema v3 includes per-clip exposure/contrast/saturation and keyframe data, plus per-track gain. Tauri routes these through undoable Rust commands. The webview preview applies color filters, track attenuation, and linear keyframe interpolation for position, scale, rotation, and opacity.

Playback and effects are still webview-based rather than a frame-accurate Rust playback/render engine. Export, full audio mixing, asset-provider APIs, AI features, and Linux release packaging remain to be built. Keep GTK available until the Tauri app has a native render pipeline, workflow coverage, and Linux package validation.
