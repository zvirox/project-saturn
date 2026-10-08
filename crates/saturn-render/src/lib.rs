use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_editing_services as ges;
use gstreamer_editing_services::prelude::*;
use gstreamer_pbutils as gst_pbutils;
use saturn_core::Tick;
use saturn_core::project::{Clip, MediaKind, ProjectDocument, TrackKind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static GST_INIT: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderProgress {
    pub fraction_percent: u8,
    pub phase: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderRange {
    pub start: Tick,
    pub end: Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Mp4,
    WebM,
}

impl OutputFormat {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("mp4") => Ok(Self::Mp4),
            Some("webm") => Ok(Self::WebM),
            _ => Err("Choose an .mp4 or .webm output file".into()),
        }
    }
}

/// Renders an Easy Edit Pro sequence to MP4 or WebM with GStreamer Editing Services.
/// GES objects are created, used, and dropped on the calling thread because its API is not Send.
pub fn render_project(
    document: &ProjectDocument,
    output_path: &Path,
    on_progress: impl FnMut(RenderProgress),
) -> Result<(), String> {
    render_project_range(document, output_path, None, on_progress)
}

pub fn render_project_range(
    document: &ProjectDocument,
    output_path: &Path,
    range: Option<RenderRange>,
    mut on_progress: impl FnMut(RenderProgress),
) -> Result<(), String> {
    document.validate().map_err(|error| error.to_string())?;
    let format = OutputFormat::from_path(output_path)?;
    if document
        .project
        .sequence
        .tracks
        .iter()
        .all(|track| track.clips.is_empty())
    {
        return Err("Add at least one clip to the timeline before rendering".into());
    }
    let ranged_document = match range {
        Some(range) => Some(slice_document(document, range)?),
        None => None,
    };
    let render_document = ranged_document.as_ref().unwrap_or(document);
    if render_document.project.sequence.tracks.iter().all(|track| track.clips.is_empty()) {
        return Err("The In/Out range does not contain any clips to render".into());
    }
    let output_path = absolute_output_path(output_path)?;

    GST_INIT
        .get_or_init(|| {
            gst::init().map_err(|error| format!("Could not initialize GStreamer: {error}"))?;
            ges::init().map_err(|error| {
                format!("Could not initialize GStreamer Editing Services: {error}")
            })
        })
        .clone()?;

    on_progress(RenderProgress {
        fraction_percent: 0,
        phase: "Preparing timeline",
    });
    let timeline = build_timeline(render_document)?;
    let profile = encoding_profile(format)?;
    let output_uri = gst::glib::filename_to_uri(&output_path, None)
        .map_err(|error| format!("Output path is not a valid file URI: {error}"))?;

    let pipeline = ges::Pipeline::new();
    pipeline
        .set_timeline(&timeline)
        .map_err(|error| format!("Could not connect the render timeline: {error}"))?;
    pipeline
        .set_render_settings(&output_uri, &profile)
        .map_err(|error| format!("Could not configure the render output: {error}"))?;
    pipeline
        .set_mode(ges::PipelineFlags::RENDER)
        .map_err(|error| format!("Could not set rendering mode: {error}"))?;

    let duration = timeline.duration().nseconds().max(1);
    let bus = pipeline
        .bus()
        .ok_or_else(|| "Render pipeline has no message bus".to_string())?;
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|error| format!("Could not start rendering: {error}"))?;

    let mut last_percent = 0_u8;
    let result = loop {
        if let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) {
            match message.view() {
                gst::MessageView::Eos(..) => break Ok(()),
                gst::MessageView::Error(error) => {
                    break Err(format!(
                        "Render failed: {}{}",
                        error.error(),
                        error
                            .debug()
                            .map(|debug| format!(" ({debug})"))
                            .unwrap_or_default()
                    ));
                }
                _ => {}
            }
        }
        let position = pipeline
            .query_position::<gst::ClockTime>()
            .map(|position| position.nseconds())
            .unwrap_or(0);
        let fraction_percent = ((position.saturating_mul(100) / duration).min(99)) as u8;
        if fraction_percent > last_percent {
            last_percent = fraction_percent;
            on_progress(RenderProgress {
                fraction_percent,
                phase: "Rendering",
            });
        }
    };

    let _ = pipeline.set_state(gst::State::Null);
    result?;
    on_progress(RenderProgress {
        fraction_percent: 100,
        phase: "Complete",
    });
    Ok(())
}

fn slice_document(document: &ProjectDocument, range: RenderRange) -> Result<ProjectDocument, String> {
    if range.start.0 < 0 || range.end.0 <= range.start.0 {
        return Err("Export range must have an Out point after its In point".into());
    }
    let mut sliced = document.clone();
    for track in &mut sliced.project.sequence.tracks {
        let mut clips = Vec::new();
        for original in &track.clips {
            let original_start = original.timeline_start.0;
            let original_end = original_start.saturating_add(original.duration.0);
            let intersect_start = original_start.max(range.start.0);
            let intersect_end = original_end.min(range.end.0);
            if intersect_start >= intersect_end { continue; }
            let trim = intersect_start - original_start;
            let mut clip = original.clone();
            clip.timeline_start = Tick(intersect_start - range.start.0);
            clip.source_in.0 = clip.source_in.0.saturating_add(trim);
            clip.duration = Tick(intersect_end - intersect_start);
            clip.keyframes.retain(|key| key.time.0 >= trim && key.time.0 <= trim + clip.duration.0);
            for key in &mut clip.keyframes { key.time.0 -= trim; }
            clips.push(clip);
        }
        track.clips = clips;
    }
    sliced.project.sequence.mark_in = None;
    sliced.project.sequence.mark_out = None;
    Ok(sliced)
}

fn build_timeline(document: &ProjectDocument) -> Result<ges::Timeline, String> {
    let timeline = ges::Timeline::new_audio_video();
    let Some(video_track) = timeline
        .tracks()
        .into_iter()
        .find(|track| track.track_type().contains(ges::TrackType::VIDEO))
    else {
        return Err("Could not create the GStreamer video track".into());
    };
    let Some(audio_track) = timeline
        .tracks()
        .into_iter()
        .find(|track| track.track_type().contains(ges::TrackType::AUDIO))
    else {
        return Err("Could not create the GStreamer audio track".into());
    };

    let video_caps = gst::Caps::builder("video/x-raw")
        .field("width", document.project.settings.width as i32)
        .field("height", document.project.settings.height as i32)
        .field(
            "framerate",
            gst::Fraction::new(
                document.project.settings.frame_rate.numerator as i32,
                document.project.settings.frame_rate.denominator as i32,
            ),
        )
        .build();
    video_track.update_restriction_caps(&video_caps);
    let audio_caps = gst::Caps::builder("audio/x-raw")
        .field("rate", document.project.settings.audio_sample_rate as i32)
        .build();
    audio_track.update_restriction_caps(&audio_caps);

    for track in &document.project.sequence.tracks {
        let layer = timeline.append_layer();
        for clip in &track.clips {
            let media = document
                .project
                .media
                .iter()
                .find(|media| media.id == clip.media_id)
                .ok_or_else(|| {
                    format!("Timeline clip references missing media {}", clip.media_id)
                })?;
            if !media.path.is_file() {
                return Err(format!("Source media is missing: {}", media.path.display()));
            }
            if matches!(track.kind, TrackKind::Audio) && !matches!(media.kind, MediaKind::Audio) {
                return Err(format!(
                    "{} is not audio media but is placed on an audio track",
                    media.name
                ));
            }
            let uri = gst::glib::filename_to_uri(&media.path, None).map_err(|error| {
                format!(
                    "Could not read media path {}: {error}",
                    media.path.display()
                )
            })?;
            let ges_clip = ges::UriClip::new(&uri)
                .map_err(|error| format!("Could not decode {}: {error}", media.name))?;
            ges_clip
                .set_start(clock_time(clip.timeline_start)?)
                .then_some(())
                .ok_or_else(|| format!("Could not set the start time for {}", media.name))?;
            ges_clip
                .set_inpoint(clock_time(clip.source_in)?)
                .then_some(())
                .ok_or_else(|| format!("Could not set the source in-point for {}", media.name))?;
            ges_clip
                .set_duration(clock_time(clip.duration)?)
                .then_some(())
                .ok_or_else(|| format!("Could not set the duration for {}", media.name))?;

            apply_clip_effects(
                &ges_clip,
                clip,
                track.gain_db,
                matches!(media.kind, MediaKind::Video | MediaKind::Image),
            )?;
            layer.add_clip(&ges_clip).map_err(|error| {
                format!(
                    "Could not add {} to the render timeline: {error}",
                    media.name
                )
            })?;
        }
    }
    if !timeline.commit_sync() {
        return Err("GStreamer could not finalize the render timeline".into());
    }
    Ok(timeline)
}

fn apply_clip_effects(
    clip: &ges::UriClip,
    model: &Clip,
    gain_db: f64,
    has_video: bool,
) -> Result<(), String> {
    if has_video
        && (model.color.exposure != 0.0
            || model.color.contrast != 1.0
            || model.color.saturation != 1.0)
    {
        let brightness = exposure_to_brightness(model.color.exposure);
        let effect = ges::Effect::new(&format!(
            "videobalance brightness={brightness} contrast={} saturation={}",
            model.color.contrast, model.color.saturation
        ))
        .map_err(|error| format!("Could not create clip color effect: {error}"))?;
        clip.add_top_effect(&effect, -1)
            .map_err(|error| format!("Could not apply clip color effect: {error}"))?;
    }
    if gain_db != 0.0 {
        let amplitude = 10_f64.powf(gain_db / 20.0);
        let effect = ges::Effect::new(&format!("volume volume={amplitude}"))
            .map_err(|error| format!("Could not create track gain effect: {error}"))?;
        clip.add_top_effect(&effect, -1)
            .map_err(|error| format!("Could not apply track gain effect: {error}"))?;
    }
    Ok(())
}

fn absolute_output_path(path: &Path) -> Result<PathBuf, String> {
    let filename = path
        .file_name()
        .ok_or_else(|| "Choose a valid output filename".to_string())?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create output folder: {error}"))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("Could not access output folder: {error}"))?;
    Ok(parent.join(filename))
}

fn exposure_to_brightness(stops: f64) -> f64 {
    (2_f64.powf(stops.abs()) - 1.0).copysign(stops) / (2_f64.powf(5.0) - 1.0)
}

fn clock_time(tick: Tick) -> Result<gst::ClockTime, String> {
    let nanoseconds = tick
        .as_nanoseconds()
        .ok_or_else(|| "Timeline time cannot be negative".to_string())?;
    Ok(gst::ClockTime::from_nseconds(nanoseconds))
}

fn encoding_profile(format: OutputFormat) -> Result<gst_pbutils::EncodingContainerProfile, String> {
    match format {
        OutputFormat::Mp4 => {
            let container_caps = gst::Caps::builder("video/quicktime")
                .field("variant", "iso")
                .build();
            let video_caps = gst::Caps::builder("video/x-h264").build();
            let audio_caps = gst::Caps::builder("audio/mpeg")
                .field("mpegversion", 4)
                .build();
            let video = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
                .name("H.264 video")
                .build();
            let audio = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
                .name("AAC audio")
                .build();
            let profile = gst_pbutils::EncodingContainerProfile::builder(&container_caps)
                .name("Easy Edit Pro MP4")
                .add_profile(video)
                .add_profile(audio)
                .build();
            Ok(profile)
        }
        OutputFormat::WebM => {
            let container_caps = gst::Caps::builder("video/webm").build();
            let video_caps = gst::Caps::builder("video/x-vp8").build();
            let audio_caps = gst::Caps::builder("audio/x-vorbis").build();
            let video = gst_pbutils::EncodingVideoProfile::builder(&video_caps)
                .name("VP8 video")
                .build();
            let audio = gst_pbutils::EncodingAudioProfile::builder(&audio_caps)
                .name("Vorbis audio")
                .build();
            let profile = gst_pbutils::EncodingContainerProfile::builder(&container_caps)
                .name("Easy Edit Pro WebM")
                .add_profile(video)
                .add_profile(audio)
                .build();
            Ok(profile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_is_selected_from_extension() {
        assert_eq!(
            OutputFormat::from_path(Path::new("cut.MP4")),
            Ok(OutputFormat::Mp4)
        );
        assert_eq!(
            OutputFormat::from_path(Path::new("cut.webm")),
            Ok(OutputFormat::WebM)
        );
        assert!(OutputFormat::from_path(Path::new("cut.mov")).is_err());
    }

    #[test]
    fn exposure_conversion_has_neutral_and_bounded_values() {
        assert_eq!(exposure_to_brightness(0.0), 0.0);
        assert_eq!(exposure_to_brightness(-5.0), -1.0);
        assert_eq!(exposure_to_brightness(5.0), 1.0);
    }
}
