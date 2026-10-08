# Project Saturn

Project Saturn is an open-source Linux video editor in early development. The stable 1.0.0 release is planned to launch as **Zvirox's Filmona**.

This first milestone is a native GTK4 desktop app written in Rust. It lays out the media bin, preview monitor, inspector, and multitrack timeline. The media bin can import multiple video, audio, and image files, search imported media, and display metadata discovered by GStreamer. Playback, timeline editing, project saving, and export are not implemented yet.

## Build and run

On Arch Linux, install the Rust toolchain, GTK 4.10 or newer development files, and GStreamer 1.x development files with the base/good/bad plugin sets. Then run:

```sh
cargo run
```

GTK 4 is accessed through its Rust bindings. The project is licensed under **GNU GPL version 3**; see [`LICENSE`](LICENSE).

## Current milestone

- Native desktop window and dark editor workspace
- Multi-file media import with asynchronous GStreamer metadata discovery
- Searchable media bin and metadata inspector for video, audio, and still images
- Preview monitor and transport-control placeholders
- Inspector panel and three empty timeline tracks
- GTK/Cairo timeline ruler and playhead visualization

Playback, timeline editing, and export controls remain disabled until those features are implemented.
