# Project Saturn

Project Saturn is an open-source Linux video editor in early development. The stable 1.0.0 release is planned to launch as **Zvirox's Filmona**.

This first milestone is a native GTK4 desktop shell written in Rust. It lays out the media bin, preview monitor, inspector, and multitrack timeline. Media import, playback, editing, and export are not implemented yet.

## Build and run

On Arch Linux, install the Rust toolchain and GTK 4 development files, then run:

```sh
cargo run
```

GTK 4 is accessed through its Rust bindings. The project is licensed under **GNU GPL version 3**; see [`LICENSE`](LICENSE).

## Current milestone

- Native desktop window and dark editor workspace
- Project media panel with an empty state
- Preview monitor and transport-control placeholders
- Inspector panel and three empty timeline tracks
- GTK/Cairo timeline ruler and playhead visualization

The editor controls are intentionally disabled until their corresponding media and editing features are implemented.
