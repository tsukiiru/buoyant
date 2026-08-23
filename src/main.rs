mod app;
mod config;
mod file;
mod ui;

#[tokio::main]
async fn main() -> eframe::Result {
    let native_opts = eframe::NativeOptions::default();

    eframe::run_native(
        "buoyant",
        native_opts,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
