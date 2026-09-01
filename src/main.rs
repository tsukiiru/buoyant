use std::sync::Arc;

use eframe::egui::{IconData, ViewportBuilder};

mod app;
mod config;
mod file;
mod ui;

#[tokio::main]
async fn main() -> eframe::Result {
    let native_opts = eframe::NativeOptions {
        viewport: ViewportBuilder {
            title: Some("buoyant".into()),
            app_id: Some("buoyant".into()),
            icon: Some(Arc::new(IconData::default())),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "buoyant",
        native_opts,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
