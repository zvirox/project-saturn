# Project Saturn

Project Saturn is an open-source Linux video editor in early development. The stable 1.0.0 release is planned to launch as **Zvirox's Filmona**.

Saturn's new desktop UI is being built with Tauri 2, React, TypeScript, and Vite in `apps/saturn-tauri`. The GTK4 app remains available while its media and editing workflows are migrated. Both frontends use the GTK-independent Rust core; the core and headless CLI remain Rust. The core already provides project commands, exact rational frame timing, versioned `.saturn` project files, atomic saves, and undo/redo.

## Build and run

### Tauri desktop UI

Install the Tauri 2 [Linux prerequisites](https://v2.tauri.app/start/prerequisites/), Rust, Node.js, and pnpm. From the repository root, run:

```sh
cd apps/saturn-tauri
pnpm install
pnpm tauri dev
```

The Tauri interface has the editing workspace layout, fullscreen and close controls, searchable media state, and a live connection to `saturn-core` project inspection. Media import, playback, timeline editing, rendering, and release packaging have not migrated yet.

### Existing GTK desktop app

On Arch Linux, install the Rust toolchain, GTK 4.10 or newer development files, and GStreamer 1.x development files with the base/good/bad plugin sets. Then run from the repository root:

```sh
cargo run
```

To install the GTK release binary, launcher entry, and Saturn camera icon for the current user, run `./packaging/linux/install-user.sh`.

Headless CLI examples:

```sh
cargo run -p saturn-cli -- commands
cargo run -p saturn-cli -- new ~/Videos/first-edit.saturn "First edit"
cargo run -p saturn-cli -- inspect ~/Videos/first-edit.saturn
cargo run -p saturn-cli -- add-media ~/Videos/first-edit.saturn ~/Videos/shot.mp4
```

The project is licensed under **GNU GPL version 3**; see [`LICENSE`](LICENSE).

## Current milestone

- Tauri 2 React/TypeScript/Vite desktop shell with responsive editing workspaces
- Rust Tauri command bridge backed by `saturn-core`
- GTK desktop window and dark editor workspace, retained during migration
- GTK multi-file media import with asynchronous GStreamer metadata discovery
- GTK searchable media bin and metadata inspector for video, audio, and still images
- GTK-independent core with stable command IDs, enabled-state reasons, and undo/redo
- Exact integer timeline ticks with rational frame and sample-rate conversion
- Versioned project files with migration, external media references, and atomic saves
- JSON Lines control mode for agent and automation clients: `cargo run -p saturn-cli -- control`
- GTK source monitor for selected imported media and a separate Program monitor for timeline output
- Edit, Color, Audio, Keyframes, and Export workspace shells in both desktop UIs
- Inspector panel and three empty timeline tracks in GTK
- GTK/Cairo timeline ruler and playhead visualization

Color grading, timeline audio mixing, keyframe editing, program playback, and export controls are workspace shells; their editing and render operations are not implemented yet. Source media preview works independently. See [the control protocol](docs/control-protocol.md) for the shared command interface.
