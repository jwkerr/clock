mod clock;
mod ui;

use chrono::{Local, Timelike};
use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 720.0])
            .with_min_inner_size([320.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "clock",
        options,
        Box::new(|_| {
            let now = Local::now();
            let time = clock::ClockTime::new(now.hour(), now.minute(), now.second());
            Ok(Box::new(ui::ClockApp::new(time)))
        }),
    )
}
