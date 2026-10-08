# Project Saturn

Project Saturn is an open-source Linux video editor in early development. The stable 1.0.0 release is planned to launch as **Zvirox's Filmona**.

The desktop frontend uses GTK4. Editing state and commands live in a separate GTK-independent Rust crate, shared by the app and a headless CLI. Current foundations include multi-file media import and metadata discovery, exact rational frame timing, a versioned `.saturn` project format with atomic saves and migration support, and a shared command/undo engine. Playback, clip editing, and export are not implemented yet.

## Build and run

On Arch Linux, install the Rust toolchain, GTK 4.10 or newer development files, and GStreamer 1.x development files with the base/good/bad plugin sets. Then run:

```sh
cargo run
```

Headless CLI examples:

```sh
cargo run -p saturn-cli -- commands
cargo run -p saturn-cli -- new ~/Videos/first-edit.saturn "First edit"
cargo run -p saturn-cli -- inspect ~/Videos/first-edit.saturn
cargo run -p saturn-cli -- add-media ~/Videos/first-edit.saturn ~/Videos/shot.mp4
```

GTK 4 is accessed through its Rust bindings. The project is licensed under **GNU GPL version 3**; see [`LICENSE`](LICENSE).

## Current milestone

- Native desktop window and dark editor workspace
- Multi-file media import with asynchronous GStreamer metadata discovery
- Searchable media bin and metadata inspector for video, audio, and still images
- GTK-independent core with stable command IDs, enabled-state reasons, and undo/redo
- Exact integer timeline ticks with rational frame and sample-rate conversion
- Versioned project files with migration, external media references, and atomic saves
- JSON Lines control mode for agent and automation clients: `cargo run -p saturn-cli -- control`
- Preview monitor and transport-control placeholders
- Inspector panel and three empty timeline tracks
- GTK/Cairo timeline ruler and playhead visualization

Playback, clip editing, and export controls remain disabled until those features are implemented. See [the control protocol](docs/control-protocol.md) for the shared command interface.
