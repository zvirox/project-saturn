import { type CSSProperties, type DragEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open, save } from "@tauri-apps/plugin-dialog";

type Workspace = "edit" | "color" | "audio" | "keyframes" | "export" | "library" | "ai";
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
type ColorAdjustments = { exposure: number; contrast: number; saturation: number };
type KeyframeProperty = "position_x" | "position_y" | "scale" | "rotation" | "opacity";
type Keyframe = { property: KeyframeProperty; time: number; value: number };
type Clip = {
  media_id: number;
  timeline_start: number;
  source_in: number;
  duration: number;
  color: ColorAdjustments;
  keyframes: Keyframe[];
};
type Track = { id: number; kind: "video" | "audio"; name: string; gain_db: number; clips: Clip[] };
type ProjectDocument = {
  project: {
    name: string;
    media: MediaAsset[];
    sequence: { name: string; tracks: Track[] };
    settings: { width: number; height: number; frame_rate: { numerator: number; denominator: number } };
  };
};
type ProjectStatus = { document: ProjectDocument; path: string | null; is_dirty: boolean };
type CommandStatus = { enabled: boolean; reason: string | null };
type SelectedClip = { track_id: number; clip_index: number };
type DraggedClip = SelectedClip;
type OpenverseAsset = { id: string; title: string; mediaUrl: string; sourceUrl: string; thumbnailUrl: string | null; creator: string | null; license: string | null; licenseUrl: string | null };

const TICKS_PER_SECOND = 254_016_000_000;
const PIXELS_PER_SECOND = 72;
const workspaces: { id: Workspace; label: string }[] = [
  { id: "edit", label: "Edit" },
  { id: "color", label: "Color" },
  { id: "audio", label: "Audio" },
  { id: "keyframes", label: "Keyframes" },
  { id: "export", label: "Export" },
  { id: "library", label: "Asset library" },
  { id: "ai", label: "AI assistant" },
];
const mediaFilters = [{ name: "Video, audio, and images", extensions: ["mp4", "mkv", "mov", "webm", "avi", "m4v", "mp3", "wav", "flac", "ogg", "m4a", "aac", "png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff"] }];
const projectFilters = [{ name: "Easy Edit Pro project", extensions: ["saturn"] }];
const keyframeOptions: { value: KeyframeProperty; label: string; defaultValue: number; min: number; max: number; step: number }[] = [
  { value: "position_x", label: "Position X", defaultValue: 0, min: -2000, max: 2000, step: 1 },
  { value: "position_y", label: "Position Y", defaultValue: 0, min: -2000, max: 2000, step: 1 },
  { value: "scale", label: "Scale", defaultValue: 1, min: 0.01, max: 10, step: 0.01 },
  { value: "rotation", label: "Rotation", defaultValue: 0, min: -360, max: 360, step: 1 },
  { value: "opacity", label: "Opacity", defaultValue: 1, min: 0, max: 1, step: 0.01 },
];

async function runCommand<T = unknown>(id: string, params: unknown = null): Promise<T> {
  return invoke<T>("execute_command", { request: { id, params } });
}

function Icon({ name }: { name: "fullscreen" | "restore" | "close" | "media" | "add" | "undo" | "redo" }) {
  const paths = {
    fullscreen: <path d="M7 3H3v4m10-4h4v4M3 13v4h4m10-4v4h-4" />,
    restore: <path d="M7 3h10v10M13 7H3v10h10V7Z" />,
    close: <path d="m5 5 10 10M15 5 5 15" />,
    media: <><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m9 4 2 16m5-16 2 16M3 9h18M3 15h18" /></>,
    add: <path d="M12 5v14m-7-7h14" />,
    undo: <path d="M9 14 4 9l5-5M4 9h9a6 6 0 0 1 0 12h-2" />,
    redo: <path d="m15 14 5-5-5-5m5 5h-9a6 6 0 0 0 0 12h2" />,
  };
  return <svg viewBox="0 0 20 20" aria-hidden="true">{paths[name]}</svg>;
}

function formatTime(ticks: number) {
  const seconds = Math.max(0, Math.floor(ticks / TICKS_PER_SECOND));
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  return `${String(hours).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

function keyframedValue(clip: Clip | undefined, property: KeyframeProperty, time: number, fallback: number) {
  const values = clip?.keyframes.filter((keyframe) => keyframe.property === property).sort((a, b) => a.time - b.time) ?? [];
  if (!values.length) return fallback;
  if (time <= values[0].time) return values[0].value;
  const last = values[values.length - 1];
  if (time >= last.time) return last.value;
  const nextIndex = values.findIndex((keyframe) => keyframe.time >= time);
  const previous = values[nextIndex - 1];
  const next = values[nextIndex];
  const ratio = (time - previous.time) / (next.time - previous.time);
  return previous.value + (next.value - previous.value) * ratio;
}

function App() {
  const [workspace, setWorkspace] = useState<Workspace>("edit");
  const [projectState, setProjectState] = useState<ProjectStatus | null>(null);
  const [selectedMediaId, setSelectedMediaId] = useState<number | null>(null);
  const [selectedClip, setSelectedClip] = useState<SelectedClip | null>(null);
  const [playhead, setPlayhead] = useState(0);
  const [search, setSearch] = useState("");
  const [connection, setConnection] = useState("Connecting to Rust core…");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [newProjectName, setNewProjectName] = useState("Untitled project");
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [undoStatus, setUndoStatus] = useState<CommandStatus>({ enabled: false, reason: "Nothing to undo" });
  const [redoStatus, setRedoStatus] = useState<CommandStatus>({ enabled: false, reason: "Nothing to redo" });
  const [colorDraft, setColorDraft] = useState<ColorAdjustments>({ exposure: 0, contrast: 1, saturation: 1 });
  const [gainDrafts, setGainDrafts] = useState<Record<number, number>>({});
  const [keyframeProperty, setKeyframeProperty] = useState<KeyframeProperty>("opacity");
  const [keyframeValue, setKeyframeValue] = useState("1");
  const [exportFormat, setExportFormat] = useState<"mp4" | "webm">("mp4");
  const [rendering, setRendering] = useState(false);
  const [renderProgress, setRenderProgress] = useState<{ fractionPercent: number; phase: string } | null>(null);
  const [assetQuery, setAssetQuery] = useState("");
  const [assetType, setAssetType] = useState<"images" | "audio">("images");
  const [assetResults, setAssetResults] = useState<OpenverseAsset[]>([]);
  const [aiModel, setAiModel] = useState("llama3.2:3b");
  const [aiPrompt, setAiPrompt] = useState("");
  const [aiReply, setAiReply] = useState("");
  const programVideoRef = useRef<HTMLVideoElement>(null);
  const programAudioRef = useRef<HTMLAudioElement>(null);

  const refreshProject = useCallback(async () => {
    const [status, undo, redo] = await Promise.all([
      runCommand<ProjectStatus>("project.status"),
      invoke<CommandStatus>("command_status", { id: "edit.undo" }),
      invoke<CommandStatus>("command_status", { id: "edit.redo" }),
    ]);
    setProjectState(status);
    setUndoStatus(undo);
    setRedoStatus(redo);
    setConnection("Rust core connected");
    setSelectedMediaId((current) => current ?? status.document.project.media[0]?.id ?? null);
  }, []);

  useEffect(() => {
    let active = true;
    refreshProject().catch((error: unknown) => {
      if (active) setConnection(`Rust core error: ${String(error)}`);
    });
    return () => { active = false; };
  }, [refreshProject]);

  const project = projectState?.document.project ?? null;
  const media = project?.media ?? [];
  const visibleMedia = useMemo(() => {
    const query = search.trim().toLowerCase();
    return media.filter((item) => !query || item.name.toLowerCase().includes(query) || item.path.toLowerCase().includes(query));
  }, [media, search]);
  const selectedMedia = media.find((item) => item.id === selectedMediaId) ?? null;
  const selectedTimelineClip = selectedClip
    ? project?.sequence.tracks.find((track) => track.id === selectedClip.track_id)?.clips[selectedClip.clip_index] ?? null
    : null;
  const activeClip = useMemo(() => {
    if (!project) return null;
    for (const track of project.sequence.tracks) {
      for (const clip of track.clips) {
        if (playhead >= clip.timeline_start && playhead < clip.timeline_start + clip.duration) {
          const clipMedia = media.find((item) => item.id === clip.media_id);
          if (clipMedia) return { track, clip, media: clipMedia };
        }
      }
    }
    return null;
  }, [media, playhead, project]);
  const timelineEndSeconds = Math.max(14, ...(project?.sequence.tracks ?? []).flatMap((track) => track.clips.map((clip) => (clip.timeline_start + clip.duration) / TICKS_PER_SECOND + 2)));
  const timelineWidth = timelineEndSeconds * PIXELS_PER_SECOND + 120;

  useEffect(() => {
    if (selectedTimelineClip) setColorDraft(selectedTimelineClip.color);
  }, [selectedTimelineClip]);

  useEffect(() => {
    if (!project) return;
    setGainDrafts(Object.fromEntries(project.sequence.tracks.map((track) => [track.id, track.gain_db])));
  }, [project]);

  useEffect(() => {
    const player = programVideoRef.current ?? programAudioRef.current;
    if (!player || !activeClip) return;
    const seconds = (activeClip.clip.source_in + playhead - activeClip.clip.timeline_start) / TICKS_PER_SECOND;
    if (Number.isFinite(seconds) && Math.abs(player.currentTime - seconds) > 0.35) player.currentTime = Math.max(0, seconds);
  }, [activeClip, playhead]);

  useEffect(() => {
    const player = programVideoRef.current ?? programAudioRef.current;
    if (!player || !activeClip) return;
    player.volume = Math.max(0, Math.min(1, 10 ** (activeClip.track.gain_db / 20)));
  }, [activeClip]);

  async function perform(id: string, params: unknown = null, success?: string) {
    setBusy(true);
    setNotice("");
    try {
      await runCommand(id, params);
      await refreshProject();
      if (success) setNotice(success);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function saveColor(next: ColorAdjustments) {
    if (!selectedClip) return;
    setColorDraft(next);
    await perform("clip.set_color", { ...selectedClip, ...next }, "Color settings saved");
  }

  async function saveTrackGain(trackId: number, gainDb: number) {
    setGainDrafts((current) => ({ ...current, [trackId]: gainDb }));
    await perform("track.set_gain_db", { track_id: trackId, gain_db: gainDb }, "Track gain saved");
  }

  async function saveKeyframe(property: KeyframeProperty, value: number) {
    if (!selectedClip || !selectedTimelineClip) return;
    const time = Math.max(0, Math.min(selectedTimelineClip.duration, playhead - selectedTimelineClip.timeline_start));
    await perform("clip.set_keyframe", { ...selectedClip, property, time, value }, "Keyframe saved");
  }

  async function ensureDiscardIsOkay() {
    if (!projectState?.is_dirty) return true;
    return confirm("This project has unsaved changes. Continue and discard them?", { title: "Unsaved project", kind: "warning" });
  }

  async function createProject() {
    const name = newProjectName.trim();
    if (!name) return;
    setNewProjectOpen(false);
    setSelectedMediaId(null);
    setSelectedClip(null);
    setPlayhead(0);
    await perform("project.new", { name }, "New project created");
  }

  async function openProject() {
    if (!(await ensureDiscardIsOkay())) return;
    const path = await open({ title: "Open Easy Edit Pro project", multiple: false, filters: projectFilters });
    if (typeof path !== "string") return;
    setBusy(true);
    setNotice("");
    try {
      await runCommand("file.open", { path });
      setSelectedClip(null);
      setPlayhead(0);
      await refreshProject();
      setNotice("Project opened");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function saveProject() {
    if (!project) return;
    let path = projectState?.path ?? null;
    if (!path) {
      path = await save({
        title: "Save Easy Edit Pro project",
        defaultPath: `${project.name.replace(/[\\/:*?"<>|]/g, "-")}.saturn`,
        filters: projectFilters,
      });
    }
    if (!path) return;
    await perform("file.save", { path }, "Project saved");
  }

  async function exportProject() {
    if (!project || rendering) return;
    const extension = exportFormat;
    const outputPath = await save({
      title: "Export video",
      defaultPath: `${project.name.replace(/[\\/:*?"<>|]/g, "-")}.${extension}`,
      filters: [{ name: extension === "mp4" ? "MP4 video" : "WebM video", extensions: [extension] }],
    });
    if (!outputPath) return;

    const channel = new Channel<{ fractionPercent: number; phase: string }>();
    channel.onmessage = (progress) => setRenderProgress(progress);
    setRendering(true);
    setRenderProgress({ fractionPercent: 0, phase: "Preparing timeline" });
    setNotice("");
    try {
      await invoke("start_export", { outputPath, onProgress: channel });
      setRenderProgress({ fractionPercent: 100, phase: "Complete" });
      setNotice(`Export complete: ${outputPath}`);
    } catch (error) {
      setNotice(`Export failed: ${String(error)}`);
    } finally {
      setRendering(false);
    }
  }

  async function searchAssets() {
    setBusy(true);
    setNotice("");
    try {
      const results = await invoke<OpenverseAsset[]>("search_openverse", { query: assetQuery, mediaType: assetType });
      setAssetResults(results);
      setNotice(`${results.length} openly licensed result${results.length === 1 ? "" : "s"} from Openverse`);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function addLibraryAsset(asset: OpenverseAsset) {
    setBusy(true);
    setNotice(`Downloading ${asset.title}…`);
    try {
      const path = await invoke<string>("download_openverse_asset", { asset });
      const status = await invoke<ProjectStatus>("import_media", { paths: [path] });
      setProjectState(status);
      setSelectedMediaId(status.document.project.media.at(-1)?.id ?? null);
      setWorkspace("edit");
      setNotice(`${asset.title} added. Attribution was saved beside the downloaded asset.`);
    } catch (error) {
      setNotice(`Could not add asset: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function askAi() {
    setBusy(true);
    setNotice("Waiting for the local Ollama model…");
    try {
      const response = await invoke<string>("ask_local_ai", { prompt: aiPrompt, model: aiModel });
      setAiReply(response);
      setNotice("Response received from local Ollama");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function importMedia() {
    const choice = await open({ title: "Import media into Easy Edit Pro", multiple: true, filters: mediaFilters });
    const paths = choice === null ? [] : Array.isArray(choice) ? choice : [choice];
    if (paths.length === 0) return;
    setBusy(true);
    setNotice(`Scanning ${paths.length} file${paths.length === 1 ? "" : "s"}…`);
    try {
      const status = await invoke<ProjectStatus>("import_media", { paths });
      setProjectState(status);
      const lastItem = status.document.project.media.at(-1);
      if (lastItem) setSelectedMediaId(lastItem.id);
      setSelectedClip(null);
      setNotice(`${paths.length} media file${paths.length === 1 ? "" : "s"} imported`);
      const [undo, redo] = await Promise.all([
        invoke<CommandStatus>("command_status", { id: "edit.undo" }),
        invoke<CommandStatus>("command_status", { id: "edit.redo" }),
      ]);
      setUndoStatus(undo);
      setRedoStatus(redo);
    } catch (error) {
      setNotice(`Import failed: ${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function addSelectedToTimeline() {
    if (!selectedMedia || !project) return;
    const desiredKind = selectedMedia.kind === "audio" ? "audio" : "video";
    const track = project.sequence.tracks.find((item) => item.kind === desiredKind);
    if (!track) {
      setNotice(`This project has no ${desiredKind} track`);
      return;
    }
    setBusy(true);
    try {
      const result = await runCommand<SelectedClip>("timeline.add_clip", { media_id: selectedMedia.id, track_id: track.id });
      setSelectedClip(result);
      setPlayhead(0);
      await refreshProject();
      setNotice("Clip added to timeline");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function removeSelectedClip() {
    if (!selectedClip) return;
    await perform("timeline.remove_clip", selectedClip, "Clip removed");
    setSelectedClip(null);
  }

  async function handleClipDrop(event: DragEvent<HTMLDivElement>, targetTrackId: number) {
    event.preventDefault();
    const serialized = event.dataTransfer.getData("application/x-saturn-clip");
    if (!serialized) return;
    const dragged: DraggedClip = JSON.parse(serialized) as DraggedClip;
    const scroller = event.currentTarget.closest(".timeline-scroller");
    const laneLeft = event.currentTarget.getBoundingClientRect().left;
    const scrollLeft = scroller instanceof HTMLElement ? scroller.scrollLeft : 0;
    const seconds = Math.max(0, (event.clientX - laneLeft + scrollLeft) / PIXELS_PER_SECOND);
    const frameRate = project?.settings.frame_rate ?? { numerator: 30, denominator: 1 };
    const frameTicks = Math.round(TICKS_PER_SECOND * frameRate.denominator / frameRate.numerator);
    const snappedStart = Math.round(seconds * TICKS_PER_SECOND / frameTicks) * frameTicks;
    setBusy(true);
    try {
      const result = await runCommand<SelectedClip & { moved: boolean }>("timeline.move_clip", { ...dragged, target_track_id: targetTrackId, timeline_start: snappedStart });
      setSelectedClip({ track_id: result.track_id, clip_index: result.clip_index });
      await refreshProject();
      setNotice("Clip moved");
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function toggleFullscreen() {
    try {
      const appWindow = getCurrentWindow();
      const isFullscreen = await appWindow.isFullscreen();
      await appWindow.setFullscreen(!isFullscreen);
      setFullscreen(!isFullscreen);
    } catch (error) {
      setNotice(`Could not change window mode: ${String(error)}`);
    }
  }

  function renderInspector() {
    switch (workspace) {
      case "color": {
        const updateColorField = (field: keyof ColorAdjustments, value: number) => {
          const next = { ...colorDraft, [field]: value };
          setColorDraft(next);
          void saveColor(next);
        };
        return <><h2>Color grading</h2><p>{selectedTimelineClip ? "Adjust the selected timeline clip." : "Select a timeline clip to grade it."}</p>
          <label>Exposure <input type="range" min="-5" max="5" step="0.1" disabled={!selectedClip} value={colorDraft.exposure} onChange={(event) => setColorDraft((value) => ({ ...value, exposure: Number(event.target.value) }))} onPointerUp={(event) => updateColorField("exposure", Number(event.currentTarget.value))} onKeyUp={(event) => updateColorField("exposure", Number(event.currentTarget.value))} /></label>
          <label>Contrast <input type="range" min="0" max="2" step="0.01" disabled={!selectedClip} value={colorDraft.contrast} onChange={(event) => setColorDraft((value) => ({ ...value, contrast: Number(event.target.value) }))} onPointerUp={(event) => updateColorField("contrast", Number(event.currentTarget.value))} onKeyUp={(event) => updateColorField("contrast", Number(event.currentTarget.value))} /></label>
          <label>Saturation <input type="range" min="0" max="2" step="0.01" disabled={!selectedClip} value={colorDraft.saturation} onChange={(event) => setColorDraft((value) => ({ ...value, saturation: Number(event.target.value) }))} onPointerUp={(event) => updateColorField("saturation", Number(event.currentTarget.value))} onKeyUp={(event) => updateColorField("saturation", Number(event.currentTarget.value))} /></label>
          <small>Preview uses webview color filters. Values are saved with the project.</small></>;
      }
      case "audio":
        return <><h2>Audio mixer</h2><p>Track attenuation is saved with the project and applied to program playback.</p>{project?.sequence.tracks.map((track) => <label className="mixer-control" key={track.id}>{track.name}<input type="range" min="-60" max="0" step="0.5" value={gainDrafts[track.id] ?? track.gain_db} onChange={(event) => setGainDrafts((values) => ({ ...values, [track.id]: Number(event.target.value) }))} onPointerUp={(event) => void saveTrackGain(track.id, Number(event.currentTarget.value))} onKeyUp={(event) => void saveTrackGain(track.id, Number(event.currentTarget.value))} /><small>{(gainDrafts[track.id] ?? track.gain_db).toFixed(1)} dB</small></label>)}</>;
      case "keyframes": {
        const option = keyframeOptions.find((item) => item.value === keyframeProperty)!;
        const keyframes = selectedTimelineClip?.keyframes ?? [];
        return <><h2>Keyframes</h2><p>{selectedTimelineClip ? "Add or remove clip property keys at the current playhead." : "Select a timeline clip to edit its keyframes."}</p>
          <label>Property <select disabled={!selectedClip} value={keyframeProperty} onChange={(event) => { const value = event.target.value as KeyframeProperty; setKeyframeProperty(value); setKeyframeValue(String(keyframeOptions.find((item) => item.value === value)?.defaultValue ?? 0)); }}>{keyframeOptions.map((item) => <option value={item.value} key={item.value}>{item.label}</option>)}</select></label>
          <div className="control">Clip time <span>{selectedTimelineClip ? formatTime(Math.max(0, Math.min(selectedTimelineClip.duration, playhead - selectedTimelineClip.timeline_start))) : "—"}</span></div>
          <label>Value <input type="number" min={option.min} max={option.max} step={option.step} disabled={!selectedClip} value={keyframeValue} onChange={(event) => setKeyframeValue(event.target.value)} /></label>
          <button className="primary" disabled={!selectedClip || !Number.isFinite(Number(keyframeValue))} onClick={() => void saveKeyframe(keyframeProperty, Number(keyframeValue))}>Set keyframe</button>
          <div className="keyframe-list">{keyframes.map((keyframe) => <div className="control" key={`${keyframe.property}-${keyframe.time}`}><span>{keyframeOptions.find((item) => item.value === keyframe.property)?.label} · {formatTime(keyframe.time)} · {keyframe.value}</span><button aria-label="Remove keyframe" onClick={() => selectedClip && void perform("clip.remove_keyframe", { ...selectedClip, property: keyframe.property, time: keyframe.time }, "Keyframe removed")}>×</button></div>)}</div>
          <small>Keyframes are stored in the project. Animated playback interpolation is not connected yet.</small></>;
      }
      case "export":
        return <><h2>Export</h2><p>Render the sequence using its project resolution and frame rate.</p>
          <label>Format <select value={exportFormat} disabled={rendering} onChange={(event) => setExportFormat(event.target.value as "mp4" | "webm")}><option value="mp4">MP4 · H.264 + AAC</option><option value="webm">WebM · VP8 + Vorbis</option></select></label>
          <div className="control">Resolution <span>{project ? `${project.settings.width} × ${project.settings.height}` : "—"}</span></div>
          <div className="control">Frame rate <span>{project ? `${project.settings.frame_rate.numerator}/${project.settings.frame_rate.denominator} fps` : "—"}</span></div>
          <button className="primary" disabled={!project || rendering || !project.sequence.tracks.some((track) => track.clips.length > 0)} onClick={() => void exportProject()}>{rendering ? "Rendering…" : "Render video"}</button>
          {renderProgress && <div className="render-progress"><progress max={100} value={renderProgress.fractionPercent} /><small>{renderProgress.phase} · {renderProgress.fractionPercent}%</small></div>}
          <small>Native GStreamer render. Color controls and track gain are included; animated keyframes are not yet rendered.</small></>;
      case "library":
        return <><h2>FOSS asset library</h2><p>Search openly licensed media indexed by Openverse. Check each source page and license before publishing your work.</p>
          <label>Search <input value={assetQuery} maxLength={200} placeholder="Try nature, city, music…" onChange={(event) => setAssetQuery(event.target.value)} /></label>
          <label>Media type <select value={assetType} onChange={(event) => setAssetType(event.target.value as typeof assetType)}><option value="images">Images</option><option value="audio">Audio</option></select></label>
          <button className="primary" disabled={busy || !assetQuery.trim()} onClick={() => void searchAssets()}>Search Openverse</button>
          <div className="asset-results">{assetResults.map((asset) => <article className="asset-result" key={`${asset.id}-${asset.mediaUrl}`}>
            {asset.thumbnailUrl && <img src={asset.thumbnailUrl} alt="" loading="lazy" />}
            <strong>{asset.title}</strong><small>{asset.creator ? `By ${asset.creator}` : "Creator not listed"} · {asset.license ?? "Check source license"}</small>
            <div className="asset-actions"><a href={asset.sourceUrl} target="_blank" rel="noreferrer">Source and license</a><button disabled={busy} onClick={() => void addLibraryAsset(asset)}>Add to media bin</button></div>
          </article>)}</div></>;
      case "ai":
        return <><h2>Local AI assistant</h2><p>Easy Edit Pro sends prompts only to Ollama running on this computer. Install Ollama and pull a model before use.</p>
          <label>Ollama model <input value={aiModel} maxLength={100} onChange={(event) => setAiModel(event.target.value)} /></label>
          <label>Prompt <textarea rows={5} maxLength={12000} value={aiPrompt} placeholder="Ask for editing ideas, a shot order, or help planning a sequence…" onChange={(event) => setAiPrompt(event.target.value)} /></label>
          <button className="primary" disabled={busy || !aiPrompt.trim() || !aiModel.trim()} onClick={() => void askAi()}>Ask local model</button>
          {aiReply && <pre className="ai-reply">{aiReply}</pre>}
          <small>AI suggestions do not modify the timeline or project automatically.</small></>;
      default:
        if (selectedTimelineClip && selectedClip) {
          const mediaForClip = media.find((item) => item.id === selectedTimelineClip.media_id);
          return <><h2>Clip inspector</h2><p>{mediaForClip?.name ?? "Timeline clip"}</p><div className="control">Start <span>{formatTime(selectedTimelineClip.timeline_start)}</span></div><div className="control">Duration <span>{formatTime(selectedTimelineClip.duration)}</span></div><div className="control">Source in <span>{formatTime(selectedTimelineClip.source_in)}</span></div><button className="danger-button" onClick={removeSelectedClip}>Remove clip</button></>;
        }
        return <><h2>Inspector</h2><p>Select media or a clip to inspect its properties.</p>{selectedMedia ? <><div className="control">Type <span>{selectedMedia.kind}</span></div><div className="control">Duration <span>{selectedMedia.duration ? formatTime(selectedMedia.duration) : "—"}</span></div><div className="control">Dimensions <span>{selectedMedia.width && selectedMedia.height ? `${selectedMedia.width} × ${selectedMedia.height}` : "—"}</span></div></> : <div className="control">No selection</div>}</>;
    }
  }

  const renderPreview = (item: MediaAsset | null, isProgram = false) => {
    if (!item) return <div className="monitor-stage"><strong>{isProgram ? "Program monitor" : "Source monitor"}</strong><small>{isProgram ? "Add a clip to the timeline to preview the sequence" : "Select imported media to preview it here"}</small></div>;
    const source = convertFileSrc(item.path);
    const color = isProgram ? activeClip?.clip.color : null;
    const clipTime = activeClip ? playhead - activeClip.clip.timeline_start : 0;
    const exposure = color?.exposure ?? 0;
    const contrast = color?.contrast ?? 1;
    const saturation = color?.saturation ?? 1;
    const positionX = keyframedValue(activeClip?.clip, "position_x", clipTime, 0);
    const positionY = keyframedValue(activeClip?.clip, "position_y", clipTime, 0);
    const scale = keyframedValue(activeClip?.clip, "scale", clipTime, 1);
    const rotation = keyframedValue(activeClip?.clip, "rotation", clipTime, 0);
    const opacity = keyframedValue(activeClip?.clip, "opacity", clipTime, 1);
    const transform = `translate(${positionX / (project?.settings.width ?? 1920) * 100}%, ${positionY / (project?.settings.height ?? 1080) * 100}%) scale(${scale}) rotate(${rotation}deg)`;
    const visualStyle: CSSProperties = { filter: `brightness(${2 ** exposure}) contrast(${contrast}) saturate(${saturation})`, transform: isProgram ? transform : undefined, opacity: isProgram ? opacity : undefined, transformOrigin: "center" };
    if (item.kind === "video") return <div className="preview-stage"><video ref={isProgram ? programVideoRef : undefined} src={source} style={visualStyle} controls={!isProgram} playsInline onTimeUpdate={isProgram ? (event) => setPlayhead(activeClip!.clip.timeline_start + activeClip!.clip.source_in + event.currentTarget.currentTime * TICKS_PER_SECOND) : undefined} /></div>;
    if (item.kind === "audio") return <div className="preview-stage audio-preview"><audio ref={isProgram ? programAudioRef : undefined} src={source} controls playsInline onTimeUpdate={isProgram ? (event) => setPlayhead(activeClip!.clip.timeline_start + activeClip!.clip.source_in + event.currentTarget.currentTime * TICKS_PER_SECOND) : undefined} /></div>;
    if (item.kind === "image") return <div className="preview-stage"><img src={source} alt={item.name} style={visualStyle} /></div>;
    return <div className="monitor-stage"><strong>Preview unavailable</strong><small>Easy Edit Pro could not identify this media type.</small></div>;
  };

  return (
    <div className="app-shell" tabIndex={-1} onKeyDown={(event) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      if (key === "s") { event.preventDefault(); void saveProject(); }
      if (key === "o") { event.preventDefault(); void openProject(); }
      if (key === "i") { event.preventDefault(); void importMedia(); }
      if (key === "n") { event.preventDefault(); if (projectState?.is_dirty) void ensureDiscardIsOkay().then((ok) => ok && setNewProjectOpen(true)); else setNewProjectOpen(true); }
      if (key === "z" && event.shiftKey) { event.preventDefault(); void perform("edit.redo"); }
      else if (key === "z") { event.preventDefault(); void perform("edit.undo"); }
    }}>
      <header className="app-header">
        <div className="brand"><img className="brand-mark" src="/saturn-camera.png" alt="Easy Edit Pro camera logo" /><div><strong>Easy Edit Pro</strong><small>{project?.name ?? "Loading project…"}{projectState?.is_dirty ? " •" : ""}</small></div></div>
        <nav className="file-actions" aria-label="Project actions">
          <button onClick={() => { void ensureDiscardIsOkay().then((ok) => { if (ok) { setNewProjectName("Untitled project"); setNewProjectOpen(true); } }); }}>New</button>
          <button onClick={() => void openProject()}>Open</button>
          <button onClick={() => void saveProject()}>Save</button>
          <button onClick={() => void importMedia()} disabled={busy}>Import</button>
          <button className="history-button" onClick={() => void perform("edit.undo")} disabled={!undoStatus.enabled} title={undoStatus.reason ?? "Undo"}><Icon name="undo" /></button>
          <button className="history-button" onClick={() => void perform("edit.redo")} disabled={!redoStatus.enabled} title={redoStatus.reason ?? "Redo"}><Icon name="redo" /></button>
        </nav>
        <nav className="workspaces" aria-label="Editing workspaces">
          {workspaces.map((item) => <button key={item.id} className={`workspace ${workspace === item.id ? "active" : ""}`} aria-pressed={workspace === item.id} onClick={() => setWorkspace(item.id)}>{item.label}</button>)}
        </nav>
        <div className="header-actions">
          <button className="icon-button" onClick={() => void toggleFullscreen()} aria-label={fullscreen ? "Leave fullscreen" : "Enter fullscreen"} title={fullscreen ? "Leave fullscreen" : "Enter fullscreen"}><Icon name={fullscreen ? "restore" : "fullscreen"} /></button>
          <button className="icon-button close" onClick={() => void getCurrentWindow().close()} aria-label="Close Easy Edit Pro" title="Close"><Icon name="close" /></button>
        </div>
      </header>

      <main className="editor">
        <aside className="media-panel">
          <div className="panel-tabs"><span className="selected">Project</span><span>Media Browser</span><span>Libraries</span></div>
          <div className="panel-heading"><strong>Project media</strong><button className="primary" onClick={() => void importMedia()} disabled={busy}>＋ Import media</button></div>
          <input className="search" type="search" placeholder="Search project media" aria-label="Search project media" value={search} onChange={(event) => setSearch(event.target.value)} />
          {visibleMedia.length > 0 ? <div className="media-list">{visibleMedia.map((item) => <button className={`media-item ${selectedMediaId === item.id ? "selected" : ""}`} key={item.id} onClick={() => { setSelectedMediaId(item.id); setSelectedClip(null); }}><Icon name="media" /><span><strong>{item.name}</strong><small>{item.kind} · {item.width && item.height ? `${item.width} × ${item.height}` : item.path}</small></span></button>)}</div> : <div className="empty-media"><span className="media-icon"><Icon name="media" /></span><strong>{search ? "No matching media" : "No media imported"}</strong><small>{search ? "Try another search." : "Import video, audio, and images to begin."}</small></div>}
        </aside>

        <section className="work-area">
          <div className="monitors">
            <section className="monitor"><div className="panel-tabs"><span className="selected">Source</span></div><div className="monitor-name">{selectedMedia?.name ?? "No source selected"}</div>{renderPreview(selectedMedia)}</section>
            <section className="monitor"><div className="panel-tabs"><span className="selected">Program</span></div><div className="monitor-name">{project?.sequence.name ?? "Sequence 1"}</div>{renderPreview(activeClip?.media ?? null, true)}<div className="transport"><button disabled={!activeClip} aria-label="Previous frame" onClick={() => setPlayhead((value) => Math.max(0, value - Math.round(TICKS_PER_SECOND * (project?.settings.frame_rate.denominator ?? 1) / (project?.settings.frame_rate.numerator ?? 30))))}>◀</button><button disabled={!activeClip} aria-label="Play or pause" onClick={() => { const player = programVideoRef.current ?? programAudioRef.current; if (!player) return; if (player.paused) void player.play(); else player.pause(); }}>{activeClip ? "▶/Ⅱ" : "▶"}</button><button disabled={!activeClip} aria-label="Next frame" onClick={() => setPlayhead((value) => value + Math.round(TICKS_PER_SECOND * (project?.settings.frame_rate.denominator ?? 1) / (project?.settings.frame_rate.numerator ?? 30)))}>▶|</button><code>{formatTime(playhead)}</code></div></section>
          </div>

          <section className="timeline">
            <div className="timeline-toolbar"><strong>Timeline</strong><span>{project?.name ?? "Loading project…"}</span><button className="primary" disabled={!selectedMedia || busy} onClick={() => void addSelectedToTimeline()}><Icon name="add" /> Add selected</button><button disabled={!selectedClip || busy} onClick={() => void removeSelectedClip()}>Remove clip</button></div>
            <div className="timeline-scroller">
              <div className="timeline-content" style={{ width: `${timelineWidth}px` }}>
                <div className="timeline-ruler"><span className="track-spacer">Time</span><div className="ruler-lane">{Array.from({ length: 16 }, (_, index) => <span key={index}>{formatTime(index * TICKS_PER_SECOND * 5)}</span>)}</div></div>
                {(project?.sequence.tracks ?? []).map((track) => <div className="track-row" key={track.id}>
                  <div className="track-label"><b>{track.name}</b><span>{track.kind}</span></div>
                  <div className="track-lane" onDragOver={(event) => event.preventDefault()} onDrop={(event) => void handleClipDrop(event, track.id)}>
                    {track.clips.map((clip, index) => {
                      const item = media.find((entry) => entry.id === clip.media_id);
                      const selected = selectedClip?.track_id === track.id && selectedClip.clip_index === index;
                      return <div className={`timeline-clip ${selected ? "selected" : ""} ${track.kind}`} key={`${track.id}-${clip.media_id}-${index}-${clip.timeline_start}`} style={{ left: `${clip.timeline_start / TICKS_PER_SECOND * PIXELS_PER_SECOND}px`, width: `${Math.max(34, clip.duration / TICKS_PER_SECOND * PIXELS_PER_SECOND)}px` }} draggable onDragStart={(event) => event.dataTransfer.setData("application/x-saturn-clip", JSON.stringify({ track_id: track.id, clip_index: index }))} onClick={() => { setSelectedClip({ track_id: track.id, clip_index: index }); setPlayhead(clip.timeline_start); }} title={`${item?.name ?? "Clip"} · drag to move`}><strong>{item?.name ?? "Missing media"}</strong></div>;
                    })}
                    <div className="playhead" style={{ left: `${playhead / TICKS_PER_SECOND * PIXELS_PER_SECOND}px` }} />
                  </div>
                </div>)}
              </div>
            </div>
          </section>
        </section>

        <aside className="tool-panel" aria-live="polite">{renderInspector()}</aside>
      </main>
      <footer className="status-bar"><span>{busy ? "Working…" : notice || "Easy Edit Pro · React + TypeScript"}</span><span>{connection}</span></footer>
      {newProjectOpen && <div className="dialog-backdrop" role="presentation"><form className="new-project-dialog" onSubmit={(event) => { event.preventDefault(); void createProject(); }}><h2>New project</h2><label>Project name<input autoFocus value={newProjectName} onChange={(event) => setNewProjectName(event.target.value)} /></label><div className="dialog-actions"><button type="button" onClick={() => setNewProjectOpen(false)}>Cancel</button><button className="primary" type="submit" disabled={!newProjectName.trim()}>Create</button></div></form></div>}
    </div>
  );
}

export default App;
