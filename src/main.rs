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
            .icon_name("saturn-camera")
            .default_width(1280)
            .default_height(800)
            .resizable(true)
            .build();
        window.set_size_request(900, 620);

        ui::build(&window);
        window.present();
    });

    app.run();
}
