mod app;
mod config;
mod file_types;
mod path;
mod theme;
mod types;

use app::Buoyant;
use std::env::args;

fn main() -> iced::Result {
    let input = args().nth(1).unwrap_or_default();
    // for optional starting path as an argument

    iced::application(move || Buoyant::new(&input), Buoyant::update, Buoyant::view)
        .subscription(Buoyant::subscription)
        .title("buoyant")
        .run()
}
