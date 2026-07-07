mod file_system;

use eframe::egui::{self, Button, Color32, RichText};
use std::{env, path, process};

fn main() -> eframe::Result {
    let native_opts = eframe::NativeOptions::default();

    eframe::run_native(
        "buoyant - egui",
        native_opts,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    path: path::PathBuf,
    is_hidden: bool,
}

struct App {
    current_path: path::PathBuf,
    explorer_entries: Vec<Entry>,
}

impl App {
    fn new(__cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.fetch_files();

        app
    }

    fn fetch_files(&mut self) {
        self.explorer_entries.clear();

        let fetch_current_files = file_system::read_dir(&self.current_path);

        if let Err(err) = &fetch_current_files {
            println!("{}", err);
        }

        let current_paths = fetch_current_files.unwrap();
        current_paths.iter().for_each(|p| {
            let name = p.file_name().unwrap().to_str().unwrap().to_string();
            self.explorer_entries.push(Entry {
                name,
                path: p.to_path_buf(),
                is_hidden: file_system::is_hidden(p),
            });
        });
    }

    fn nav(&mut self, to: &path::Path) {
        if to.is_file() {
            let res = process::Command::new("xdg-open").arg(to).spawn();
            if let Err(err) = res {
                println!("{}", err);
            }
        }

        if !to.is_dir() {
            return;
        }

        self.current_path = to.to_path_buf();
        self.fetch_files();
    }

    fn nav_back(&mut self) {
        self.current_path.pop();
        self.fetch_files();
    }
}

impl Default for App {
    fn default() -> Self {
        App {
            current_path: env::home_dir().unwrap(),
            explorer_entries: Vec::with_capacity(20),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("da explorer");

            egui::ScrollArea::vertical().show(ui, |ui| {
                let text = RichText::new("<--").size(14.0);
                let button = ui.add(Button::new(text).fill(egui::Color32::TRANSPARENT));

                if button.clicked() {
                    self.nav_back();
                }

                for entry in self.explorer_entries.clone() {
                    let mut text = RichText::new(&entry.name).size(14.0);

                    if entry.is_hidden {
                        text = text.color(Color32::WHITE.gamma_multiply(0.5));
                    }

                    let button = Button::new(text).fill(egui::Color32::TRANSPARENT);

                    if ui.add(button).clicked() {
                        self.nav(&entry.path);
                    }
                }
            })
        });
    }
}
