mod file_system;

use eframe::egui::{
    self, Button, Color32, Context, Id, Label, Modal, RichText, Sense, TextEdit, Vec2,
};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use crate::file_system::BANNED_CHARACTERS;

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
    path: PathBuf,
    is_hidden: bool,
}

struct App {
    ctx: Context,
    current_path: PathBuf,
    explorer_entries: Vec<Entry>,
    modals: Modals,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = App {
            ctx: cc.egui_ctx.clone(),
            current_path: env::home_dir().unwrap(),
            modals: Modals::default(),
            explorer_entries: Vec::with_capacity(20),
        };
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

    fn nav(&mut self, to: &Path) {
        if to.is_file() {
            let res = Command::new("xdg-open").arg(to).spawn();
            if let Err(err) = res {
                println!("{}", err);
            }
            return;
        }

        if !to.is_dir() && !to.is_symlink() {
            return;
        }

        self.current_path = to.to_path_buf();
        self.fetch_files();
    }

    fn nav_back(&mut self) {
        self.current_path.pop();
        self.fetch_files();
    }

    fn delete(&mut self, path: &Path) {
        file_system::delete(path);
        self.fetch_files();
    }

    fn create(&mut self, mode: bool) {
        // true: file
        // false: folder

        if mode {
            let overlay = self.modals.create_file.as_mut().unwrap();
            let mut content = overlay.content.trim();
            if content.starts_with("/") {
                content = &content[1..];
            }

            let try_create = file_system::create(&self.current_path, Path::new(content), true);

            if let Some(error) = try_create {
                overlay.error.clear();
                overlay.error.push_str(error);
                return;
            }
        } else {
            let overlay = self.modals.create_folder.as_mut().unwrap();
            let mut content = overlay.content.trim();
            if content.starts_with("/") {
                content = &content[1..];
            }

            let try_create = file_system::create(&self.current_path, Path::new(content), false);

            if let Some(error) = try_create {
                overlay.error.clear();
                overlay.error.push_str(error);
                return;
            }
        }

        self.close_modal(ModalType::CreateFile);
        self.close_modal(ModalType::CreateFolder);
        self.fetch_files();
        return;
    }

    fn rename(&mut self) {
        let rename_modal_opt = self.modals.rename.as_mut();
        if rename_modal_opt.is_none() {
            return;
        }

        let rename_modal = rename_modal_opt.unwrap();

        let content = rename_modal.content.trim();
        for char in BANNED_CHARACTERS {
            if content.contains(char) {
                rename_modal.error.clear();
                rename_modal
                    .error
                    .push_str("Containing invalid characters!");
                return;
            }
        }

        file_system::rename(&rename_modal.path.as_ref().unwrap(), &rename_modal.content);
        self.close_modal(ModalType::Rename);
        self.fetch_files();
    }

    fn new_modal(&mut self, modal: ModalType, path: Option<&Path>) {
        match modal {
            ModalType::Rename => {
                let path = path.unwrap();
                self.modals.rename = Some(UModal {
                    content: path.file_name().unwrap().to_str().unwrap().to_string(),
                    path: Some(path.to_path_buf()),
                    error: String::new(),
                })
            }
            ModalType::CreateFile => {
                self.modals.create_file = Some(UModal {
                    content: String::new(),
                    path: None,
                    error: String::new(),
                })
            }
            ModalType::CreateFolder => {
                self.modals.create_folder = Some(UModal {
                    content: String::new(),
                    path: None,
                    error: String::new(),
                })
            }
        }
    }

    fn update_modal(&mut self, modal: ModalType, new_content: String) {
        match modal {
            ModalType::Rename => self.modals.rename.as_mut().unwrap().content = new_content,
            ModalType::CreateFile => {
                self.modals.create_file.as_mut().unwrap().content = new_content
            }

            ModalType::CreateFolder => {
                self.modals.create_folder.as_mut().unwrap().content = new_content
            }
        }
    }

    fn close_modal(&mut self, modal: ModalType) {
        match modal {
            ModalType::Rename => self.modals.rename = None,
            ModalType::CreateFile => self.modals.create_file = None,
            ModalType::CreateFolder => self.modals.create_folder = None,
        };
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("da explorer");

            let interact = egui::ScrollArea::vertical().show(ui, |ui| {
                let bg_response = ui.interact(
                    ui.available_rect_before_wrap(),
                    Id::new("explorer-area"),
                    Sense::click(),
                );
                let text = RichText::new("<--").size(14.0);
                let mut button = ui.add(Button::new(text).fill(egui::Color32::TRANSPARENT));
                button.set_intrinsic_size(Vec2::new(400.0, 20.0));

                if button.clicked() {
                    self.nav_back();
                }

                for entry in self.explorer_entries.clone() {
                    let mut text = RichText::new(&entry.name).size(14.0);

                    if entry.is_hidden {
                        text = text.color(Color32::WHITE.gamma_multiply(0.5));
                    }

                    let mut button = ui.add(Button::new(text).fill(egui::Color32::TRANSPARENT));
                    button.set_intrinsic_size(Vec2::new(400.0, 20.0));

                    button.context_menu(|ui| {
                        ui.label("stuff");
                        if ui.button("rename").clicked() {
                            self.new_modal(ModalType::Rename, Some(&entry.path));
                        }
                        if ui.button("delete").clicked() {
                            self.delete(&entry.path);
                        }
                    });

                    if button.clicked() {
                        self.nav(&entry.path);
                    }
                }

                return bg_response;
            });

            interact.inner.context_menu(|ui| {
                ui.label("stuff i think");
                if ui.button("create file").clicked() {
                    self.new_modal(ModalType::CreateFile, None);
                }
                if ui.button("create folder").clicked() {
                    self.new_modal(ModalType::CreateFolder, None);
                }
            });
        });

        if let Some(modal) = &self.modals.rename {
            let modal_widget = Modal::new(Id::new("rename_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.heading("renaming");
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalType::Rename, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.rename();
                    } else {
                        self.close_modal(ModalType::Rename);
                    }
                }

                input.request_focus();
            });
        }

        if let Some(modal) = &self.modals.create_file {
            let modal_widget = Modal::new(Id::new("create_file_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.heading("creating file");
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalType::CreateFile, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.create(true);
                    } else {
                        self.close_modal(ModalType::CreateFile);
                    }
                }

                input.request_focus();
            });
        }

        if let Some(modal) = &self.modals.create_folder {
            let modal_widget = Modal::new(Id::new("create_folder_modal"));
            let mut content = modal.content.clone();
            let error = &modal.error.clone();

            modal_widget.show(&self.ctx.clone(), |ui| {
                ui.heading("creating folder");
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    self.update_modal(ModalType::CreateFolder, content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.create(false);
                    } else {
                        self.close_modal(ModalType::CreateFolder);
                    }
                }

                input.request_focus();
            });
        }
    }
}

struct Modals {
    rename: Option<UModal>,
    create_file: Option<UModal>,
    create_folder: Option<UModal>,
}

impl Default for Modals {
    fn default() -> Self {
        Modals {
            rename: None,
            create_file: None,
            create_folder: None,
        }
    }
}

enum ModalType {
    Rename,
    CreateFile,
    CreateFolder,
}

struct UModal {
    path: Option<PathBuf>,
    content: String,
    error: String,
}

impl UModal {}
