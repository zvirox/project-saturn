use gstreamer_pbutils::Discoverer;
use gtk::cairo::{Context, FontSlant, FontWeight};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Box as GtkBox, Button, DrawingArea, FileDialog, FileFilter, Frame, Label,
    ListBox, ListBoxRow, MenuButton, Orientation, Paned, ScrolledWindow, Separator,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;

pub fn build(window: &ApplicationWindow) {
    install_styles();

    let inspector = InspectorWidgets::new();
    let (result_sender, result_receiver) = mpsc::channel();
    let discoverer = Discoverer::new(gstreamer::ClockTime::from_seconds(30))
        .expect("GStreamer Discoverer should initialize after GStreamer");
    let _discovered_handler = discoverer.connect_discovered(move |_discoverer, info, error| {
        let uri = info.uri().to_string();
        let result = describe_media(info, error.map(|error| error.to_string()));
        let _ = result_sender.send((uri, result));
    });
    discoverer.start();

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.add_css_class("app-root");

    let media_panel = build_media_panel(window, discoverer, result_receiver, inspector.clone());
    root.append(&build_header(window));

    let workspace = Paned::new(Orientation::Horizontal);
    workspace.set_wide_handle(true);
    workspace.set_position(278);
    workspace.set_start_child(Some(&media_panel));
    workspace.set_end_child(Some(&build_editor_area(inspector)));
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

fn build_header(window: &ApplicationWindow) -> GtkBox {
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

    header.append(&build_menu_bar(window));

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

fn build_menu_bar(window: &ApplicationWindow) -> GtkBox {
    let bar = GtkBox::new(Orientation::Horizontal, 2);
    bar.add_css_class("menu-bar");

    let file = gtk::gio::Menu::new();
    file.append(Some("New Project"), Some("win.new-project"));
    file.append(Some("Open Project…"), Some("win.open-project"));
    file.append(Some("Save Project"), Some("win.save-project"));
    file.append(Some("Save Project As…"), Some("win.save-project-as"));
    let file_actions = gtk::gio::Menu::new();
    file_actions.append(Some("Import Media…"), Some("win.import-media"));
    file.append_section(None, &file_actions);
    let export = gtk::gio::Menu::new();
    export.append(Some("Export Project…"), Some("win.export"));
    file.append_section(None, &export);

    let edit = gtk::gio::Menu::new();
    edit.append(Some("Undo"), Some("win.undo"));
    edit.append(Some("Redo"), Some("win.redo"));
    let clipboard = gtk::gio::Menu::new();
    clipboard.append(Some("Cut"), Some("win.cut"));
    clipboard.append(Some("Copy"), Some("win.copy"));
    clipboard.append(Some("Paste"), Some("win.paste"));
    clipboard.append(Some("Delete"), Some("win.delete"));
    edit.append_section(None, &clipboard);
    edit.append(Some("Select All"), Some("win.select-all"));

    let tools = gtk::gio::Menu::new();
    tools.append(Some("Audio Mixer"), Some("win.audio-mixer"));
    tools.append(Some("Scene Detection"), Some("win.scene-detection"));
    tools.append(Some("AI Tools"), Some("win.ai-tools"));
    tools.append(Some("Keyboard Shortcuts"), Some("win.keyboard-shortcuts"));

    let view = gtk::gio::Menu::new();
    view.append(Some("Full Screen"), Some("win.fullscreen"));
    view.append(Some("Zoom In"), Some("win.zoom-in"));
    view.append(Some("Zoom Out"), Some("win.zoom-out"));
    view.append(Some("Reset Zoom"), Some("win.reset-zoom"));
    view.append(Some("Reset Workspace Layout"), Some("win.reset-layout"));

    let help = gtk::gio::Menu::new();
    help.append(Some("Project Saturn User Guide"), Some("win.user-guide"));
    help.append(Some("Report an Issue"), Some("win.report-issue"));
    help.append(Some("About Project Saturn"), Some("win.about"));

    for (title, menu) in [
        ("File", file),
        ("Edit", edit),
        ("Tools", tools),
        ("View", view),
        ("Help", help),
    ] {
        let button = MenuButton::new();
        button.set_label(title);
        button.set_menu_model(Some(&menu));
        button.add_css_class("menu-button");
        bar.append(&button);
    }

    let fullscreen_action = gtk::gio::SimpleAction::new("fullscreen", None);
    let window_for_fullscreen = window.clone();
    fullscreen_action.connect_activate(move |_, _| {
        if window_for_fullscreen.is_fullscreen() {
            window_for_fullscreen.unfullscreen();
        } else {
            window_for_fullscreen.fullscreen();
        }
    });
    window.add_action(&fullscreen_action);

    let about_action = gtk::gio::SimpleAction::new("about", None);
    let window_for_about = window.clone();
    about_action.connect_activate(move |_, _| show_about_window(&window_for_about));
    window.add_action(&about_action);

    for name in [
        "new-project",
        "open-project",
        "save-project",
        "save-project-as",
        "export",
        "undo",
        "redo",
        "cut",
        "copy",
        "paste",
        "delete",
        "select-all",
        "audio-mixer",
        "scene-detection",
        "ai-tools",
        "keyboard-shortcuts",
        "zoom-in",
        "zoom-out",
        "reset-zoom",
        "reset-layout",
        "user-guide",
        "report-issue",
    ] {
        let unavailable = gtk::gio::SimpleAction::new(name, None);
        unavailable.set_enabled(false);
        window.add_action(&unavailable);
    }

    bar
}

fn show_about_window(parent: &ApplicationWindow) {
    let about = gtk::Window::builder()
        .title("About Project Saturn")
        .transient_for(parent)
        .modal(true)
        .default_width(360)
        .default_height(190)
        .build();
    let content = GtkBox::new(Orientation::Vertical, 10);
    content.set_margin_top(24);
    content.set_margin_bottom(20);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let title = Label::new(Some("Project Saturn"));
    title.add_css_class("about-title");
    content.append(&title);
    let version = Label::new(Some("Open-source Linux video editor · v0.1.0"));
    version.add_css_class("muted-label");
    content.append(&version);
    let brand = Label::new(Some("Planned stable release name: Zvirox’s Filmona"));
    brand.add_css_class("muted-label");
    brand.set_wrap(true);
    content.append(&brand);

    let close = Button::with_label("Close");
    close.set_halign(gtk::Align::End);
    let about_for_close = about.clone();
    close.connect_clicked(move |_| about_for_close.close());
    content.append(&close);
    about.set_child(Some(&content));
    about.present();
}

#[derive(Clone)]
struct InspectorWidgets {
    title: Label,
    kind: Label,
    duration: Label,
    dimensions: Label,
    location: Label,
}

impl InspectorWidgets {
    fn new() -> Self {
        Self {
            title: Label::new(Some("Nothing selected")),
            kind: Label::new(Some("—")),
            duration: Label::new(Some("—")),
            dimensions: Label::new(Some("—")),
            location: Label::new(Some("—")),
        }
    }

    fn show(&self, name: &str, path: &str, metadata: &MediaMetadata) {
        self.title.set_text(name);
        self.kind.set_text(&metadata.kind);
        self.duration.set_text(&metadata.duration);
        self.dimensions.set_text(&metadata.dimensions);
        self.location.set_text(path);
    }
}

#[derive(Clone, Default)]
struct MediaMetadata {
    kind: String,
    duration: String,
    dimensions: String,
    message: String,
}

struct MediaRow {
    name: String,
    path: String,
    subtitle: Label,
    metadata: MediaMetadata,
}

fn build_media_panel(
    window: &ApplicationWindow,
    discoverer: Discoverer,
    result_receiver: mpsc::Receiver<(String, MediaMetadata)>,
    inspector: InspectorWidgets,
) -> GtkBox {
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
    actions.append(&import_button);
    panel.append(&actions);

    let import_action = gtk::gio::SimpleAction::new("import-media", None);
    let import_button_for_action = import_button.clone();
    import_action.connect_activate(move |_, _| import_button_for_action.emit_clicked());
    window.add_action(&import_action);
    if let Some(application) = window.application() {
        application.set_accels_for_action("win.import-media", &["<Primary>i"]);
    }

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search project media"));
    panel.append(&search);

    let separator = Separator::new(Orientation::Horizontal);
    panel.append(&separator);

    let list = ListBox::new();
    list.add_css_class("media-list");
    list.set_selection_mode(gtk::SelectionMode::Single);

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

    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    stack.add_named(&empty, Some("empty"));
    let scroll = ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&list));
    stack.add_named(&scroll, Some("media"));
    stack.set_visible_child_name("empty");
    panel.append(&stack);

    let paths = Rc::new(RefCell::new(Vec::<String>::new()));
    let rows = Rc::new(RefCell::new(HashMap::<String, MediaRow>::new()));
    let selected_uri = Rc::new(RefCell::new(None::<String>));

    let dialog_parent = window.clone();
    let list_for_import = list.clone();
    let stack_for_import = stack.clone();
    let paths_for_import = paths.clone();
    let rows_for_import = rows.clone();
    let inspector_for_import = inspector.clone();
    let selected_uri_for_import = selected_uri.clone();
    let discoverer_for_import = discoverer.clone();
    import_button.connect_clicked(move |_| {
        let dialog = FileDialog::builder()
            .title("Import media into Project Saturn")
            .build();

        let all_media = FileFilter::new();
        all_media.set_name(Some("Supported media"));
        for pattern in [
            "*.mp4", "*.mkv", "*.mov", "*.webm", "*.avi", "*.m4v", "*.mp3", "*.wav", "*.flac",
            "*.ogg", "*.m4a", "*.aac", "*.png", "*.jpg", "*.jpeg", "*.webp", "*.bmp", "*.tif",
            "*.tiff",
        ] {
            all_media.add_pattern(pattern);
        }
        let any_file = FileFilter::new();
        any_file.set_name(Some("All files"));
        any_file.add_pattern("*");
        let filters = gtk::gio::ListStore::new::<FileFilter>();
        filters.append(&all_media);
        filters.append(&any_file);
        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&all_media));

        let list = list_for_import.clone();
        let stack = stack_for_import.clone();
        let paths = paths_for_import.clone();
        let rows = rows_for_import.clone();
        let inspector = inspector_for_import.clone();
        let selected_uri = selected_uri_for_import.clone();
        let discoverer = discoverer_for_import.clone();
        dialog.open_multiple(
            Some(&dialog_parent),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                let files = match result {
                    Ok(files) => files,
                    Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => return,
                    Err(error) => {
                        eprintln!("Could not open media picker: {error}");
                        return;
                    }
                };

                for index in 0..files.n_items() {
                    let Some(file) = files.item(index).and_downcast::<gtk::gio::File>() else {
                        continue;
                    };
                    let uri = file.uri().to_string();
                    if rows.borrow().contains_key(&uri) {
                        continue;
                    }
                    let path = file
                        .path()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| uri.clone());
                    let name = file
                        .basename()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.clone());

                    let row = ListBoxRow::new();
                    row.add_css_class("media-row");
                    let contents = GtkBox::new(Orientation::Horizontal, 10);
                    contents.set_margin_top(8);
                    contents.set_margin_bottom(8);
                    contents.set_margin_start(8);
                    contents.set_margin_end(8);
                    let glyph = Label::new(Some("▧"));
                    glyph.add_css_class("media-glyph");
                    contents.append(&glyph);
                    let labels = GtkBox::new(Orientation::Vertical, 3);
                    labels.set_hexpand(true);
                    let title = Label::new(Some(&name));
                    title.add_css_class("media-name");
                    title.set_xalign(0.0);
                    title.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
                    let subtitle = Label::new(Some("Reading metadata…"));
                    subtitle.add_css_class("muted-label");
                    subtitle.set_xalign(0.0);
                    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    labels.append(&title);
                    labels.append(&subtitle);
                    contents.append(&labels);
                    row.set_child(Some(&contents));
                    list.append(&row);

                    paths.borrow_mut().push(uri.clone());
                    rows.borrow_mut().insert(
                        uri.clone(),
                        MediaRow {
                            name,
                            path,
                            subtitle,
                            metadata: MediaMetadata {
                                message: "Reading metadata…".into(),
                                ..Default::default()
                            },
                        },
                    );
                    stack.set_visible_child_name("media");

                    if let Err(error) = discoverer.discover_uri_async(&uri) {
                        if let Some(media_row) = rows.borrow_mut().get_mut(&uri) {
                            media_row.metadata.message = format!("Could not scan: {error}");
                            media_row.subtitle.set_text(&media_row.metadata.message);
                        }
                    }
                }
                if paths.borrow().len() > 0 {
                    list.select_row(list.row_at_index(0).as_ref());
                    if let Some(uri) = paths.borrow().first() {
                        *selected_uri.borrow_mut() = Some(uri.clone());
                        show_selected(uri, &rows, &inspector);
                    }
                }
            },
        );
    });

    let paths_for_selection = paths.clone();
    let selected_uri_for_selection = selected_uri.clone();
    let rows_for_selection = rows.clone();
    let inspector_for_selection = inspector.clone();
    list.connect_row_selected(move |_list, row| {
        let Some(row) = row else {
            return;
        };
        let index = row.index() as usize;
        let Some(uri) = paths_for_selection.borrow().get(index).cloned() else {
            return;
        };
        *selected_uri_for_selection.borrow_mut() = Some(uri.clone());
        show_selected(&uri, &rows_for_selection, &inspector_for_selection);
    });

    let rows_for_results = rows.clone();
    let selected_for_results = selected_uri.clone();
    let inspector_for_results = inspector.clone();
    search.connect_search_changed(move |search| {
        let query = search.text().to_string().to_lowercase();
        for (index, uri) in paths.borrow().iter().enumerate() {
            if let Some(row) = list.row_at_index(index as i32) {
                let name = rows
                    .borrow()
                    .get(uri)
                    .map(|entry| entry.name.to_lowercase())
                    .unwrap_or_default();
                row.set_visible(query.is_empty() || name.contains(&query));
            }
        }
    });
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok((uri, metadata)) = result_receiver.try_recv() {
            if let Some(row) = rows_for_results.borrow_mut().get_mut(&uri) {
                row.subtitle.set_text(&metadata.message);
                row.metadata = metadata;
                if selected_for_results.borrow().as_deref() == Some(uri.as_str()) {
                    show_selected(&uri, &rows_for_results, &inspector_for_results);
                }
            }
        }
        gtk::glib::ControlFlow::Continue
    });
    panel
}

fn show_selected(
    uri: &str,
    rows: &Rc<RefCell<HashMap<String, MediaRow>>>,
    inspector: &InspectorWidgets,
) {
    if let Some(row) = rows.borrow().get(uri) {
        inspector.show(&row.name, &row.path, &row.metadata);
    }
}

fn describe_media(
    info: &gstreamer_pbutils::DiscovererInfo,
    error: Option<String>,
) -> MediaMetadata {
    let videos = info.video_streams();
    let audios = info.audio_streams();
    let (kind, dimensions) = if let Some(video) = videos.first() {
        (
            if video.is_image() { "Image" } else { "Video" }.to_string(),
            format!("{} × {}", video.width(), video.height()),
        )
    } else if !audios.is_empty() {
        ("Audio".to_string(), "Audio only".to_string())
    } else {
        ("Image or unknown media".to_string(), "—".to_string())
    };
    let duration = info
        .duration()
        .map(|duration| {
            let seconds = duration.seconds();
            format!(
                "{:02}:{:02}:{:02}",
                seconds / 3600,
                (seconds / 60) % 60,
                seconds % 60
            )
        })
        .unwrap_or_else(|| "Still image / unknown".into());
    let message = error
        .map(|error| format!("Metadata partial · {error}"))
        .unwrap_or_else(|| format!("{kind} · {duration}"));
    MediaMetadata {
        kind,
        duration,
        dimensions,
        message,
    }
}

fn build_editor_area(inspector: InspectorWidgets) -> GtkBox {
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
    upper.set_end_child(Some(&build_inspector_panel(inspector)));
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

fn build_inspector_panel(inspector: InspectorWidgets) -> GtkBox {
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

    inspector.title.add_css_class("inspector-media-title");
    inspector.title.set_xalign(0.0);
    inspector.title.set_wrap(true);
    panel.append(&inspector.title);
    panel.append(&inspector_row("Type", &inspector.kind));
    panel.append(&inspector_row("Duration", &inspector.duration));
    panel.append(&inspector_row("Dimensions", &inspector.dimensions));

    let location_heading = Label::new(Some("Location"));
    location_heading.add_css_class("inspector-key");
    location_heading.set_xalign(0.0);
    location_heading.set_margin_top(8);
    panel.append(&location_heading);
    inspector.location.add_css_class("muted-label");
    inspector.location.set_xalign(0.0);
    inspector.location.set_wrap(true);
    panel.append(&inspector.location);
    panel
}

fn inspector_row(name: &str, value: &Label) -> GtkBox {
    let row = GtkBox::new(Orientation::Vertical, 3);
    let key = Label::new(Some(name));
    key.add_css_class("inspector-key");
    key.set_xalign(0.0);
    value.add_css_class("inspector-value");
    value.set_xalign(0.0);
    row.append(&key);
    row.append(value);
    row
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
