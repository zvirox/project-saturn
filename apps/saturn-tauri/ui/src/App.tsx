import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Workspace = "edit" | "color" | "audio" | "keyframes" | "export";
type MediaKind = "video" | "audio" | "image" | "unknown";
type MediaAsset = {
  id: number;
  name: string;
  path: string;
  kind: MediaKind;
  duration: number | null;
  width: number | null;
  height: number | null;
};
type ProjectDocument = {
  project: {
    name: string;
    media: MediaAsset[];
    sequence: { name: string };
    settings: { width: number; height: number; frame_rate: { numerator: number; denominator: number } };
  };
};

const workspaces: { id: Workspace; label: string }[] = [
  { id: "edit", label: "Edit" },
  { id: "color", label: "Color" },
  { id: "audio", label: "Audio" },
  { id: "keyframes", label: "Keyframes" },
  { id: "export", label: "Export" },
];

function Icon({ name }: { name: "fullscreen" | "restore" | "close" | "media" }) {
  if (name === "close") {
    return <svg viewBox="0 0 20 20" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15" /></svg>;
  }
  if (name === "media") {
    return <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m9 4 2 16m5-16 2 16M3 9h18M3 15h18" /></svg>;
  }
  if (name === "restore") {
    return <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M7 3h10v10M13 7H3v10h10V7Z" /></svg>;
  }
  return <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M7 3H3v4m10-4h4v4M3 13v4h4m10-4v4h-4" /></svg>;
}

function App() {
  const [workspace, setWorkspace] = useState<Workspace>("edit");
  const [project, setProject] = useState<ProjectDocument["project"] | null>(null);
  const [search, setSearch] = useState("");
  const [connection, setConnection] = useState("Connecting to Rust core…");
  const [fullscreen, setFullscreen] = useState(false);
  const [windowError, setWindowError] = useState("");

  useEffect(() => {
    let active = true;
    invoke<ProjectDocument>("execute_command", {
      request: { id: "project.inspect", params: null },
    })
      .then((document) => {
        if (!active) return;
        setProject(document.project);
        setConnection("Rust core connected");
      })
      .catch((error: unknown) => {
        if (active) setConnection(`Rust core error: ${String(error)}`);
      });
    return () => { active = false; };
  }, []);

  const visibleMedia = useMemo(() => {
    const query = search.trim().toLowerCase();
    return (project?.media ?? []).filter((item) =>
      !query || item.name.toLowerCase().includes(query) || item.path.toLowerCase().includes(query),
    );
  }, [project?.media, search]);

  async function toggleFullscreen() {
    try {
      const appWindow = getCurrentWindow();
      const isFullscreen = await appWindow.isFullscreen();
      await appWindow.setFullscreen(!isFullscreen);
      setFullscreen(!isFullscreen);
      setWindowError("");
    } catch (error) {
      setWindowError(`Could not change window mode: ${String(error)}`);
    }
  }

  function renderInspector() {
    switch (workspace) {
      case "color":
        return <><h2>Color grading</h2><p>Primary controls for the selected timeline clip.</p><div className="control">Lift <span>◉</span></div><div className="control">Gamma <span>◉</span></div><div className="control">Gain <span>◉</span></div><label>Exposure <input type="range" disabled /></label><label>Contrast <input type="range" disabled /></label><label>Saturation <input type="range" disabled /></label><small>Color processing is planned.</small></>;
      case "audio":
        return <><h2>Audio mixer</h2><p>Balance tracks and monitor levels.</p><div className="control">A1 · Dialogue <span>▏ ▎ ▍ ▌</span></div><div className="control">A2 · Music <span>▏ ▎ ▍ ▌</span></div><div className="control">A3 · Effects <span>▏ ▎ ▍ ▌</span></div><div className="control">Master <span>▏ ▎ ▍ ▌</span></div><small>Mixer controls activate with timeline playback.</small></>;
      case "keyframes":
        return <><h2>Keyframes</h2><p>Animate clip properties over time.</p>{["Position", "Scale", "Rotation", "Opacity"].map((item) => <div className="control" key={item}>{item}<span>◇</span></div>)}<small>Clip animation is planned.</small></>;
      case "export":
        return <><h2>Export</h2><p>Choose a delivery format for your sequence.</p><label>Format <select disabled><option>MP4 · H.264</option></select></label><label>Resolution <select disabled><option>1920 × 1080</option></select></label><label>Frame rate <select disabled><option>30 fps</option></select></label><button disabled>Render video</button><small>Rendering is planned.</small></>;
      default:
        return <><h2>Inspector</h2><p>Select media or a clip to inspect its properties.</p><div className="control">Type <span>—</span></div><div className="control">Duration <span>—</span></div><div className="control">Dimensions <span>—</span></div></>;
    }
  }

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand"><img className="brand-mark" src="/saturn-camera.png" alt="Saturn camera logo" /><div><strong>Project Saturn</strong><small>{project?.name ?? "Loading project…"}</small></div></div>
        <nav className="workspaces" aria-label="Editing workspaces">
          {workspaces.map((item) => <button key={item.id} className={`workspace ${workspace === item.id ? "active" : ""}`} aria-pressed={workspace === item.id} onClick={() => setWorkspace(item.id)}>{item.label}</button>)}
        </nav>
        <div className="header-actions">
          <button className="icon-button" onClick={toggleFullscreen} aria-label={fullscreen ? "Leave fullscreen" : "Enter fullscreen"} title={fullscreen ? "Leave fullscreen" : "Enter fullscreen"}><Icon name={fullscreen ? "restore" : "fullscreen"} /></button>
          <button className="icon-button close" onClick={() => getCurrentWindow().close()} aria-label="Close Project Saturn" title="Close"><Icon name="close" /></button>
        </div>
      </header>

      <main className="editor">
        <aside className="media-panel">
          <div className="panel-tabs"><span className="selected">Project</span><span>Media Browser</span><span>Libraries</span></div>
          <div className="panel-heading"><strong>Project media</strong><button className="primary" disabled title="Media import is not connected in this migration preview">＋ Import media</button></div>
          <input className="search" type="search" placeholder="Search project media" aria-label="Search project media" value={search} onChange={(event) => setSearch(event.target.value)} />
          {visibleMedia.length > 0 ? <div className="media-list">{visibleMedia.map((item) => <button className="media-item" key={item.id}><Icon name="media" /><span><strong>{item.name}</strong><small>{item.kind} · {item.width && item.height ? `${item.width} × ${item.height}` : item.path}</small></span></button>)}</div> : <div className="empty-media"><span className="media-icon"><Icon name="media" /></span><strong>{search ? "No matching media" : "No media imported"}</strong><small>{search ? "Try another search." : "Import video, audio, and images to begin."}</small></div>}
        </aside>

        <section className="work-area">
          <div className="monitors">
            <section className="monitor"><div className="panel-tabs"><span className="selected">Source</span></div><div className="monitor-name">No source selected</div><div className="monitor-stage"><strong>Source monitor</strong><small>Preview imported media here</small></div><div className="transport"><button disabled aria-label="Previous frame">◀</button><button disabled aria-label="Play">▶</button><button disabled aria-label="Next frame">▶|</button><code>00:00:00:00</code></div></section>
            <section className="monitor"><div className="panel-tabs"><span className="selected">Program</span></div><div className="monitor-name">{project?.sequence.name ?? "Sequence 1"}</div><div className="monitor-stage"><strong>Program monitor</strong><small>Timeline output appears here</small></div><div className="transport"><button disabled aria-label="Previous frame">◀</button><button disabled aria-label="Play">▶</button><button disabled aria-label="Next frame">▶|</button><code>00:00:00:00</code></div></section>
          </div>

          <section className="timeline">
            <div className="timeline-toolbar"><strong>Timeline</strong><span>{project?.name ?? "Loading project…"}</span><button disabled>＋ Track</button></div>
            <div className="timeline-ruler"><span>00:00</span><span>00:05</span><span>00:10</span><span>00:15</span><span>00:20</span></div>
            <div className="track"><b>V2</b><span>Video overlay</span></div><div className="track"><b>V1</b><span>Main video</span></div><div className="track"><b>A1</b><span>Audio</span></div>
          </section>
        </section>

        <aside className="tool-panel" aria-live="polite">{renderInspector()}</aside>
      </main>
      <footer className="status-bar"><span>Saturn Tauri · React + TypeScript</span><span>{windowError || connection}</span></footer>
    </div>
  );
}

export default App;
