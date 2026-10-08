# Easy Edit Pro

Easy Edit Pro is an open-source Linux desktop video editor in early development. The app was previously called Project Saturn; its Rust crates, repository URL, and existing `.saturn` project format retain those internal identifiers for compatibility.

The Tauri desktop UI uses React, TypeScript, and Vite in `apps/saturn-tauri`; the GTK4 frontend remains available. Both frontends use the GTK-independent Rust core, which provides project commands, exact rational frame timing, versioned `.saturn` project files, atomic saves, and undo/redo.

## Build and run

### Tauri desktop UI

Install the Tauri 2 [Linux prerequisites](https://v2.tauri.app/start/prerequisites/), Rust, Node.js, pnpm, and GStreamer Editing Services development/runtime packages with the H.264/AAC and VP8/Vorbis plugins. From the repository root, run:

```sh
cd apps/saturn-tauri
pnpm install
pnpm tauri dev
```

The Tauri interface supports project creation/open/save, native multi-file media import, GStreamer metadata discovery, a searchable media bin, source preview, timeline clip add/move/remove, program preview, and shared Rust undo/redo commands. The native renderer uses GStreamer Editing Services to export MP4 (H.264/AAC) or WebM (VP8/Vorbis) using the project resolution and frame rate. It applies clip color controls and track gain; animated keyframes are currently preview-only. Export progress appears in the Export workspace.

The Asset Library searches Openverse for openly licensed images and audio, downloads selected media into Easy Edit Pro's application data folder, and saves a neighboring attribution JSON file. Review each source and license before distributing a project. The AI Assistant can send prompts to a local Ollama instance at `http://127.0.0.1:11434`; install Ollama and pull a model such as `llama3.2:3b` first. Prompts and responses stay on the local machine.

The interface bundles the Inter Variable font for consistent typography. Inter is licensed under the SIL Open Font License; see [`assets/fonts/OFL.txt`](assets/fonts/OFL.txt).

### Linux packages

The Tauri bundler is configured for Debian (`.deb`) and RPM (`.rpm`) packages. On Linux, build them with:

```sh
cd apps/saturn-tauri
pnpm install
pnpm tauri build
```

The package build needs the Tauri/WebKitGTK build prerequisites, GStreamer Editing Services development files, GStreamer runtime plugins, and the Debian/RPM packaging tools. The `Linux packages` GitHub Actions workflow builds both package formats on manual dispatch or a `v*` tag. Package dependencies declare GStreamer and Tauri runtime libraries; the encoder/decoder plugin packages are included as dependencies as well.

### Existing GTK desktop app

On Arch Linux, install the Rust toolchain, GTK 4.10 or newer development files, and GStreamer 1.x development files with the base/good/bad plugin sets. Then run from the repository root:

```sh
cargo run
```

To install the GTK release binary, launcher entry, Inter font, and camera app icon for the current user, run `./packaging/linux/install-user.sh`.

Headless CLI examples:

```sh
cargo run -p saturn-cli -- commands
cargo run -p saturn-cli -- new ~/Videos/first-edit.saturn "First edit"
cargo run -p saturn-cli -- inspect ~/Videos/first-edit.saturn
cargo run -p saturn-cli -- add-media ~/Videos/first-edit.saturn ~/Videos/shot.mp4
```

The project is licensed under **GNU GPL version 3**; see [`LICENSE`](LICENSE).

## Current milestone

- Easy Edit Pro Tauri 2 React/TypeScript/Vite desktop shell with responsive editing workspaces and bundled Inter Variable typography
- Rust Tauri command bridge backed by `saturn-core`, including project status and timeline editing commands
- Native project and media dialogs, asynchronous GStreamer metadata discovery, and asset-protocol media access
- Tauri source and program monitors, media-bin preview, timeline clip operations, and undo/redo
- Easy Edit Pro GTK desktop window and dark editor workspace, retained alongside the Tauri frontend
- GTK multi-file media import with asynchronous GStreamer metadata discovery
- GTK searchable media bin and metadata inspector for video, audio, and still images
- GTK-independent core with stable command IDs, enabled-state reasons, and undo/redo
- Exact integer timeline ticks with rational frame and sample-rate conversion
- Versioned project files with migration, external media references, and atomic saves
- JSON Lines control mode for agent and automation clients: `cargo run -p saturn-cli -- control`
- GTK source monitor for selected imported media and a separate Program monitor for timeline output
- Edit, Color, Audio, Keyframes, and Export workspaces in both desktop UIs; Color, Audio, and Keyframes controls persist and affect Tauri preview
- Native GStreamer Editing Services rendering from the Export workspace to MP4 or WebM, with progress and error reporting
- Openverse image/audio search and download with license/creator/source attribution sidecar files
- Local Ollama prompt integration, opt-in through the AI Assistant workspace
- Debian and RPM package configuration plus a GitHub Actions build workflow
- Inspector panel and three empty timeline tracks in GTK
- GTK/Cairo timeline ruler and playhead visualization

The Tauri Color workspace adjusts clip exposure/contrast/saturation, Audio controls persist per-track gain and apply attenuation during playback, and Keyframes adds/removes position/scale/rotation/opacity keys with linear preview interpolation. Keyframe transforms remain preview-only in the current native renderer. Track gain and basic clip color grading are rendered, while advanced audio mixing, additional asset providers, and timeline-aware AI actions remain future work. See [the control protocol](docs/control-protocol.md) for the shared command interface.
