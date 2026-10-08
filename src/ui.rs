use gtk::cairo::{Context, FontSlant, FontWeight};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Box as GtkBox, Button, DrawingArea, Frame, Label, Orientation, Paned,
    Separator,
};

pub fn build(window: &ApplicationWindow) {
    install_styles();

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.add_css_class("app-root");

    root.append(&build_header());

    let workspace = Paned::new(Orientation::Horizontal);
    workspace.set_wide_handle(true);
    workspace.set_position(278);
    workspace.set_start_child(Some(&build_media_panel()));
    workspace.set_end_child(Some(&build_editor_area()));
    workspace.set_vexpand(true);
    root.append(&workspace);

    root.append(&build_status_bar());
    window.set_child(Some(&root));
}

fn install_styles() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("style.css"));

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_header() -> GtkBox {
    let header = GtkBox::new(Orientation::Horizontal, 12);
    header.add_css_class("app-header");
    header.set_margin_start(14);
    header.set_margin_end(14);
    header.set_margin_top(8);
    header.set_margin_bottom(8);

    let mark = Label::new(Some("PS"));
    mark.add_css_class("brand-mark");
    header.append(&mark);

    let title_group = GtkBox::new(Orientation::Vertical, 1);
    let title = Label::new(Some("Project Saturn"));
    title.add_css_class("brand-title");
    title.set_xalign(0.0);
    let subtitle = Label::new(Some("Untitled project"));
    subtitle.add_css_class("muted-label");
    subtitle.set_xalign(0.0);
    title_group.append(&title);
    title_group.append(&subtitle);
    header.append(&title_group);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header.append(&spacer);

    let project_button = Button::with_label("Project settings");
    project_button.add_css_class("quiet-button");
    project_button.set_tooltip_text(Some("Project settings will be added in a later milestone"));
    header.append(&project_button);

    let export_button = Button::with_label("Export");
    export_button.add_css_class("accent-button");
    export_button.set_sensitive(false);
    export_button.set_tooltip_text(Some("Export arrives after the editing and render pipeline"));
    header.append(&export_button);

    header
}

fn build_media_panel() -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 12);
    panel.add_css_class("side-panel");
    panel.set_width_request(250);
    panel.set_margin_start(14);
    panel.set_margin_top(14);
    panel.set_margin_bottom(14);
    panel.set_margin_end(7);

    let heading = Label::new(Some("Project media"));
    heading.add_css_class("section-title");
    heading.set_xalign(0.0);
    panel.append(&heading);

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    let import_button = Button::with_label("＋  Import media");
    import_button.add_css_class("accent-button");
    import_button.set_sensitive(false);
    import_button.set_tooltip_text(Some("Media import is the next development milestone"));
    actions.append(&import_button);
    panel.append(&actions);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search project media"));
    search.set_sensitive(false);
    panel.append(&search);

    let separator = Separator::new(Orientation::Horizontal);
    panel.append(&separator);

    let empty = GtkBox::new(Orientation::Vertical, 8);
    empty.add_css_class("empty-state");
    empty.set_vexpand(true);
    empty.set_valign(gtk::Align::Center);

    let icon = Label::new(Some("▧"));
    icon.add_css_class("empty-icon");
    empty.append(&icon);

    let empty_title = Label::new(Some("Your media will appear here"));
    empty_title.add_css_class("empty-title");
    empty.append(&empty_title);

    let empty_hint = Label::new(Some("Import video, audio, and images to begin."));
    empty_hint.add_css_class("muted-label");
    empty_hint.set_wrap(true);
    empty_hint.set_justify(gtk::Justification::Center);
    empty.append(&empty_hint);

    panel.append(&empty);
    panel
}

fn build_editor_area() -> GtkBox {
    let area = GtkBox::new(Orientation::Vertical, 0);
    area.add_css_class("editor-area");
    area.set_margin_top(14);
    area.set_margin_bottom(14);
    area.set_margin_start(7);
    area.set_margin_end(14);

    let upper = Paned::new(Orientation::Horizontal);
    upper.set_wide_handle(true);
    upper.set_position(790);
    upper.set_start_child(Some(&build_preview_panel()));
    upper.set_end_child(Some(&build_inspector_panel()));
    upper.set_vexpand(true);

    let timeline = build_timeline_panel();
    timeline.set_size_request(-1, 292);

    area.append(&upper);
    area.append(&timeline);
    area
}

fn build_preview_panel() -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 10);
    panel.add_css_class("preview-panel");
    panel.set_margin_end(7);

    let top_line = GtkBox::new(Orientation::Horizontal, 8);
    let heading = Label::new(Some("Preview"));
    heading.add_css_class("section-title");
    heading.set_xalign(0.0);
    top_line.append(&heading);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    top_line.append(&spacer);

    let quality = Label::new(Some("Fit  ·  100%"));
    quality.add_css_class("muted-label");
    top_line.append(&quality);
    panel.append(&top_line);

    let stage_frame = Frame::new(None);
    stage_frame.add_css_class("preview-frame");
    stage_frame.set_hexpand(true);
    stage_frame.set_vexpand(true);

    let stage = GtkBox::new(Orientation::Vertical, 10);
    stage.add_css_class("preview-stage");
    stage.set_hexpand(true);
    stage.set_vexpand(true);
    stage.set_halign(gtk::Align::Fill);
    stage.set_valign(gtk::Align::Fill);

    let placeholder = GtkBox::new(Orientation::Vertical, 10);
    placeholder.set_halign(gtk::Align::Center);
    placeholder.set_valign(gtk::Align::Center);
    placeholder.set_vexpand(true);

    let mark = Label::new(Some("PS"));
    mark.add_css_class("preview-mark");
    placeholder.append(&mark);

    let message = Label::new(Some("Preview monitor"));
    message.add_css_class("preview-message");
    placeholder.append(&message);

    let hint = Label::new(Some("Add media to see your edit here"));
    hint.add_css_class("muted-label");
    placeholder.append(&hint);

    stage.append(&placeholder);
    stage_frame.set_child(Some(&stage));
    panel.append(&stage_frame);
    panel.append(&build_transport());
    panel
}

fn build_transport() -> GtkBox {
    let controls = GtkBox::new(Orientation::Horizontal, 8);
    controls.add_css_class("transport-bar");
    controls.set_halign(gtk::Align::Center);

    let start = Button::with_label("|◀");
    start.add_css_class("transport-button");
    start.set_sensitive(false);
    controls.append(&start);

    let previous = Button::with_label("◀");
    previous.add_css_class("transport-button");
    previous.set_sensitive(false);
    controls.append(&previous);

    let play = Button::with_label("▶");
    play.add_css_class("play-button");
    play.set_sensitive(false);
    play.set_tooltip_text(Some("Playback arrives with media import"));
    controls.append(&play);

    let next = Button::with_label("▶");
    next.add_css_class("transport-button");
    next.set_sensitive(false);
    controls.append(&next);

    let timecode = Label::new(Some("00:00:00:00"));
    timecode.add_css_class("timecode");
    timecode.set_margin_start(12);
    controls.append(&timecode);

    controls
}

fn build_inspector_panel() -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 12);
    panel.add_css_class("inspector-panel");
    panel.set_width_request(260);
    panel.set_margin_start(7);

    let heading = Label::new(Some("Inspector"));
    heading.add_css_class("section-title");
    heading.set_xalign(0.0);
    panel.append(&heading);

    let separator = Separator::new(Orientation::Horizontal);
    panel.append(&separator);

    let empty = GtkBox::new(Orientation::Vertical, 8);
    empty.add_css_class("inspector-empty");
    empty.set_vexpand(true);
    empty.set_valign(gtk::Align::Center);

    let title = Label::new(Some("Nothing selected"));
    title.add_css_class("empty-title");
    empty.append(&title);

    let hint = Label::new(Some("Select a clip to edit its properties."));
    hint.add_css_class("muted-label");
    hint.set_wrap(true);
    hint.set_justify(gtk::Justification::Center);
    empty.append(&hint);

    panel.append(&empty);
    panel
}

fn build_timeline_panel() -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 8);
    panel.add_css_class("timeline-panel");
    panel.set_margin_top(14);

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    let heading = Label::new(Some("Timeline"));
    heading.add_css_class("section-title");
    toolbar.append(&heading);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    toolbar.append(&spacer);

    let snap = Label::new(Some("Snap  ·  Zoom 100%"));
    snap.add_css_class("muted-label");
    toolbar.append(&snap);

    let add_track = Button::with_label("＋ Track");
    add_track.add_css_class("quiet-button");
    add_track.set_sensitive(false);
    toolbar.append(&add_track);
    panel.append(&toolbar);

    let ruler_row = GtkBox::new(Orientation::Horizontal, 0);
    ruler_row.add_css_class("timeline-ruler-row");
    let ruler_spacer = GtkBox::new(Orientation::Horizontal, 0);
    ruler_spacer.set_width_request(136);
    ruler_row.append(&ruler_spacer);

    let ruler = DrawingArea::new();
    ruler.set_content_height(28);
    ruler.set_hexpand(true);
    ruler.set_draw_func(draw_ruler);
    ruler_row.append(&ruler);
    panel.append(&ruler_row);

    let tracks = GtkBox::new(Orientation::Horizontal, 0);
    tracks.add_css_class("timeline-tracks");

    let labels = GtkBox::new(Orientation::Vertical, 0);
    labels.set_width_request(136);
    labels.append(&track_label("V2", "Video overlay"));
    labels.append(&track_label("V1", "Main video"));
    labels.append(&track_label("A1", "Audio"));
    tracks.append(&labels);

    let canvas = DrawingArea::new();
    canvas.set_content_height(174);
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    canvas.set_draw_func(draw_tracks);
    tracks.append(&canvas);
    panel.append(&tracks);

    panel
}

fn track_label(short_name: &str, description: &str) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.add_css_class("track-label");
    row.set_height_request(58);

    let badge = Label::new(Some(short_name));
    badge.add_css_class("track-badge");
    row.append(&badge);

    let name = Label::new(Some(description));
    name.add_css_class("track-name");
    name.set_xalign(0.0);
    row.append(&name);

    row
}

fn draw_ruler(_area: &DrawingArea, context: &Context, width: i32, height: i32) {
    context.set_source_rgb(0.105, 0.11, 0.14);
    let _ = context.paint();

    context.select_font_face("Sans", FontSlant::Normal, FontWeight::Normal);
    context.set_font_size(10.0);

    for x in (0..width).step_by(48) {
        context.set_source_rgb(0.30, 0.31, 0.36);
        context.set_line_width(1.0);
        context.move_to(x as f64 + 0.5, height as f64 - 7.0);
        context.line_to(x as f64 + 0.5, height as f64);
        let _ = context.stroke();

        if x % 192 == 0 {
            context.set_source_rgb(0.67, 0.68, 0.73);
            context.move_to(x as f64 + 5.0, 13.0);
            let seconds = x / 38;
            let label = format!("00:{seconds:02}");
            let _ = context.show_text(&label);
        }
    }
}

fn draw_tracks(_area: &DrawingArea, context: &Context, width: i32, height: i32) {
    context.set_source_rgb(0.075, 0.08, 0.105);
    let _ = context.paint();

    for track in 0..3 {
        let y = (track * 58) as f64;
        if track % 2 == 1 {
            context.set_source_rgb(0.09, 0.095, 0.12);
            context.rectangle(0.0, y, width as f64, 58.0);
            let _ = context.fill();
        }

        context.set_source_rgb(0.19, 0.20, 0.24);
        context.set_line_width(1.0);
        context.move_to(0.0, y + 57.5);
        context.line_to(width as f64, y + 57.5);
        let _ = context.stroke();
    }

    context.set_source_rgb(0.17, 0.18, 0.22);
    context.set_line_width(1.0);
    for x in (0..width).step_by(48) {
        context.move_to(x as f64 + 0.5, 0.0);
        context.line_to(x as f64 + 0.5, height as f64);
        let _ = context.stroke();
    }

    let playhead_x = 188.0;
    context.set_source_rgb(0.97, 0.43, 0.31);
    context.set_line_width(2.0);
    context.move_to(playhead_x, 0.0);
    context.line_to(playhead_x, height as f64);
    let _ = context.stroke();
    context.move_to(playhead_x - 5.0, 0.0);
    context.line_to(playhead_x + 5.0, 0.0);
    context.line_to(playhead_x, 7.0);
    context.close_path();
    let _ = context.fill();
}

fn build_status_bar() -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 8);
    bar.add_css_class("status-bar");
    bar.set_margin_start(14);
    bar.set_margin_end(14);
    bar.set_margin_top(7);
    bar.set_margin_bottom(7);

    let status = Label::new(Some("Ready for your first edit"));
    status.add_css_class("muted-label");
    bar.append(&status);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bar.append(&spacer);

    let version = Label::new(Some("Project Saturn  ·  v0.1.0-dev"));
    version.add_css_class("muted-label");
    bar.append(&version);
    bar
}
