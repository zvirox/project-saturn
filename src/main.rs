mod ui;

use gtk::Application;
use gtk::prelude::*;

fn main() {
    if let Err(error) = gstreamer::init() {
        eprintln!("Could not initialize GStreamer: {error}");
        return;
    }

    let app = Application::builder()
        .application_id("com.zvirox.ProjectSaturn")
        .build();

    app.connect_activate(|app| {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Project Saturn")
            .default_width(1440)
            .default_height(900)
            .build();

        ui::build(&window);
        window.present();
    });

    app.run();
}
