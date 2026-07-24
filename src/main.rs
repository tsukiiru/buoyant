mod app;
mod config;
mod file_system;
mod file_types;
mod icons;
mod types;

#[tokio::main]
async fn main() -> eframe::Result {
    let native_opts = eframe::NativeOptions::default();

    eframe::run_native(
        "buoyant - egui",
        native_opts,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
