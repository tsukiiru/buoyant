use chrono::{DateTime, Datelike, Utc};
use eframe::egui::{
    Align, Align2, AtomLayout, Button, CentralPanel, Color32, CornerRadius, Frame, Grid, Id, Image,
    Key, Label, LayerId, Layout, Margin, Modal, Order, Popup, PopupAnchor, ProgressBar, RectAlign,
    RichText, ScrollArea, Sense, Stroke, TextEdit, TextWrapMode, Vec2, Window,
};
use std::{ops::Sub, path::Path, sync::Arc, time::Instant};

use crate::{
    app::{App, ClipboardMode, FieldKind, OverlayKind, PanelKind, Property, ToastKind},
    file::{
        CreateType,
        icons::{IconKind, match_icon},
    },
};

impl App {
    pub fn ui(&mut self, main_ui: &mut eframe::egui::Ui) {
        let ctx = self.ctx.clone();
        let visuals = ctx.theme().default_visuals();
        let drag_layer_id = LayerId::new(Order::Tooltip, Id::new("drag"));

        // all the requests to be handled via self methods later
        let (
            mut req_set_clipboard,
            mut req_reset_clipboard,
            mut req_swap_selected,
            mut req_modify_selected,
            mut req_reset_selected,
        ) = (None, false, None, None, false);

        let (mut req_navigate_forward, mut req_navigate_backward) = (false, false);
        let mut req_create = None;
        let mut req_rename = false;
        let mut req_paste = false;
        let mut req_transfer = None;

        let (
            mut req_new_overlay,
            mut req_overlay_choice,
            mut req_close_overlay,
            mut req_update_overlay_buffer,
        ) = (None, None, false, None);

        let (
            mut req_open_field,
            mut req_close_field,
            mut req_update_field_buffer,
            mut req_logic_field,
            mut req_unfocus_field,
        ) = (None, false, None, None, false);

        let mut req_toggle_panel = None;
        //

        CentralPanel::default().show(main_ui, |ui| {
            ui.horizontal(|ui| {
                let mut button =
                    ui.add(Button::new(RichText::new("<").size(14.0)).fill(Color32::TRANSPARENT));
                button.set_intrinsic_size(Vec2::new(400.0, 20.0));

                if button.clicked() {
                    req_navigate_backward = true;
                }

                ui.label(format!("{}", self.current_path.display()));
            });

            let mut content = self.field.buffer.clone();
            let is_searching = if let Some(kind) = self.field.kind
                && kind == FieldKind::Search
            {
                true
            } else {
                false
            };

            let input = ui.add(
                TextEdit::singleline(&mut content)
                    .background_color(Color32::TRANSPARENT)
                    .hint_text(format!(
                        "({}) input search entry :3",
                        ctx.format_shortcut(&self.config.keybinds.search)
                    ))
                    .frame(Frame::NONE)
                    .desired_width(f32::INFINITY),
            );

            if input.gained_focus() && !is_searching {
                req_open_field = Some(FieldKind::Search);
            }
            if is_searching && input.changed() {
                req_update_field_buffer = Some(content);
                req_logic_field = Some(FieldKind::Search);
            }
            if is_searching && ui.input(|i| i.key_pressed(Key::Escape)) {
                req_close_field = true;
                input.surrender_focus();
            }
            if is_searching && ui.input(|i| i.key_pressed(Key::Enter)) || input.lost_focus() {
                req_unfocus_field = true;
            }
            if is_searching && self.field.focused {
                input.request_focus();
            }
            if is_searching && !self.field.focused {
                input.surrender_focus();
            }

            ui.separator();
            ui.horizontal(|ui| {
                let view = &self.config.view.explorer;
                ui.allocate_space(Vec2::new(2.0 + 16.0, 0.0));

                let mut grid = Grid::new("explorer-title-grid");
                grid = grid.min_col_width(200.0);

                grid.show(ui, |ui| {
                    view.iter().for_each(|p| {
                        ui.add(Label::new(p.to_string()).halign(Align::Min));
                    });
                });
            });

            let current_index = &self.current_index;
            let mut from: Option<Arc<usize>> = None;
            let mut to = None;

            let displaying = self.entries.displaying.clone();

            let bg_response = ui.interact(
                ui.available_rect_before_wrap(),
                Id::new("explorer-area"),
                Sense::click(),
            );

            ScrollArea::vertical().show_rows(ui, 32.0, displaying.len(), |sa, range| {
                let keybinds = &self.config.keybinds;
                let view = &self.config.view.explorer;

                for (index, entry_index) in displaying.into_iter().enumerate() {
                    let is_current_index = index == *current_index;
                    let entry_opt = self.entries.children.get(entry_index);
                    if entry_opt.is_none() || (!range.contains(&index) && !is_current_index) {
                        continue;
                    }

                    let entry = entry_opt.unwrap();

                    sa.horizontal(|h| {
                        let mut frame = Frame::NONE
                            .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
                            .corner_radius(4.0);

                        if self.selected.contains(&entry_index) {
                            frame.fill = Color32::LIGHT_GREEN.gamma_multiply(0.3);
                        }
                        if is_current_index {
                            frame.stroke.color = visuals.text_color().gamma_multiply(0.3);
                        }

                        let mut color = visuals.text_color();
                        let mut icon = &entry.file_icon;
                        if entry.is_hidden {
                            color = visuals.text_color().gamma_multiply(0.5);
                        }
                        if self.clipboard.entries.contains(&entry.path) {
                            icon = match self.clipboard.mode.as_ref().unwrap() {
                                ClipboardMode::Copy => &IconKind::Copy,
                                ClipboardMode::Cut => &IconKind::Scissors,
                            };
                            color = Color32::BLUE.gamma_multiply(0.3);
                        }

                        let fr = frame
                            .show(h, |f| {
                                let another_frame =
                                    Frame::NONE.inner_margin(Margin::symmetric(2, 8));
                                another_frame.show(f, |a| {
                                    a.add(
                                        Image::new(match_icon(icon))
                                            .fit_to_exact_size(Vec2::new(14.0, 14.0)),
                                    );
                                });

                                let mut grid = Grid::new(Id::new(("explorer-grid", &index)));

                                grid = grid.min_col_width(200.0);
                                grid.show(f, |g| {
                                    view.iter().for_each(|p| match p {
                                        Property::Name => {
                                            g.add(
                                                AtomLayout::new(&entry.name)
                                                    .wrap_mode(TextWrapMode::Truncate)
                                                    .max_width(200.0)
                                                    .fallback_text_color(color),
                                            );
                                        }
                                        Property::Accessed => {
                                            g.add(
                                                AtomLayout::new(format_date(entry.accessed))
                                                    .max_width(200.0)
                                                    .wrap_mode(TextWrapMode::Truncate)
                                                    .fallback_text_color(color),
                                            );
                                        }
                                        Property::Created => {
                                            g.add(
                                                AtomLayout::new(format_date(entry.created))
                                                    .max_width(200.0)
                                                    .wrap_mode(TextWrapMode::Truncate)
                                                    .fallback_text_color(color),
                                            );
                                        }
                                        Property::Size => {
                                            g.add(
                                                AtomLayout::new(
                                                    if let Some(size) = &entry.folder_size {
                                                        format!("{} items", size)
                                                    } else {
                                                        bytes_to_string(
                                                            entry.file_size.unwrap_or_default(),
                                                        )
                                                    },
                                                )
                                                .max_width(200.0)
                                                .wrap_mode(TextWrapMode::Truncate)
                                                .fallback_text_color(color),
                                            );
                                        }
                                        Property::Type => {
                                            g.add(
                                                AtomLayout::new(entry.file_type)
                                                    .max_width(200.0)
                                                    .wrap_mode(TextWrapMode::Truncate)
                                                    .fallback_text_color(color),
                                            );
                                        }
                                        Property::Path => {
                                            g.add(
                                                AtomLayout::new(format!(
                                                    "{}",
                                                    entry.path.display()
                                                ))
                                                .max_width(200.0)
                                                .wrap_mode(TextWrapMode::Truncate)
                                                .fallback_text_color(color),
                                            );
                                        }
                                    });
                                });
                            })
                            .response;

                        let btn_interact = h.interact(
                            fr.rect,
                            Id::new(("button", &index)),
                            Sense::click_and_drag(),
                        );
                        btn_interact.dnd_set_drag_payload(entry_index);

                        if btn_interact.drag_started() {
                            req_swap_selected = Some(index);
                        }

                        if btn_interact.dragged() {
                            let popup = Popup::new(
                                Id::new(("drag_pop", &index)),
                                ctx.clone(),
                                PopupAnchor::Pointer,
                                drag_layer_id,
                            )
                            .align(RectAlign::TOP_START)
                            .layout(Layout::left_to_right(Align::TOP));
                            popup.show(|pop| {
                                pop.add(
                                    Image::new(match_icon(&IconKind::Files))
                                        .fit_to_exact_size(Vec2::new(14.0, 14.0)),
                                );
                                pop.label(format!("files [{}]", self.selected.len()));
                            });
                        }

                        if let Some(hovered_payload) = fr.dnd_hover_payload::<usize>() {
                            if *hovered_payload != entry_index {
                                h.painter().rect_filled(
                                    fr.rect,
                                    CornerRadius::from(4.0),
                                    visuals.text_color().gamma_multiply(0.1),
                                );
                            }
                            if let Some(dragged_payload) = fr.dnd_release_payload() {
                                from = Some(dragged_payload);
                                to = Some(entry_index)
                            }
                        }

                        if is_current_index && self.scroll_signal {
                            btn_interact.scroll_to_me(None);
                            self.scroll_signal = false;
                        }

                        btn_interact.context_menu(|ui| {
                            ui.label(entry.name.clone());
                            if ui
                                .add(
                                    Button::new("rename")
                                        .shortcut_text(ctx.format_shortcut(&keybinds.rename_file)),
                                )
                                .clicked()
                            {
                                req_new_overlay = Some(OverlayKind::Rename);
                            }
                            if ui
                                .add(Button::new("delete").shortcut_text(
                                    ctx.format_shortcut(&keybinds.delete_selections),
                                ))
                                .clicked()
                            {
                                req_new_overlay = Some(OverlayKind::Delete);
                            }
                            if ui
                                .add(
                                    Button::new("cut").shortcut_text(
                                        ctx.format_shortcut(&keybinds.cut_to_clipboard),
                                    ),
                                )
                                .clicked()
                            {
                                req_set_clipboard = Some(ClipboardMode::Cut);
                            }
                            if ui
                                .add(Button::new("copy").shortcut_text(
                                    ctx.format_shortcut(&keybinds.copy_to_clipboard),
                                ))
                                .clicked()
                            {
                                req_set_clipboard = Some(ClipboardMode::Copy);
                            }
                            if ui
                                .add(
                                    Button::new("info")
                                        .shortcut_text(ctx.format_shortcut(&keybinds.view_info)),
                                )
                                .clicked()
                            {
                                req_new_overlay = Some(OverlayKind::Metadata);
                            }
                        });

                        if btn_interact.clicked() {
                            req_modify_selected = Some(index);
                        }

                        if btn_interact.double_clicked() {
                            req_navigate_forward = true;
                        }

                        if btn_interact.secondary_clicked() {
                            req_swap_selected = Some(index);
                        }
                    });
                }
            });

            if bg_response.clicked()
                && !(ui.input(|i| {
                    i.key_pressed(Key::ControlLeft)
                        && i.key_pressed(Key::ControlRight)
                        && i.key_pressed(Key::ShiftLeft)
                        && i.key_pressed(Key::ShiftRight)
                }))
            {
                self.clear_selected();
            }

            bg_response.context_menu(|ui| {
                let keybinds = &self.config.keybinds;
                ui.label("create");

                if ui
                    .add(
                        Button::new("create file")
                            .shortcut_text(ctx.format_shortcut(&keybinds.create_file_path)),
                    )
                    .clicked()
                {
                    req_new_overlay = Some(OverlayKind::CreateFile);
                }
                if ui
                    .add(
                        Button::new("create folder")
                            .shortcut_text(ctx.format_shortcut(&keybinds.create_folder_path)),
                    )
                    .clicked()
                {
                    req_new_overlay = Some(OverlayKind::CreateFolder);
                }

                ui.separator();
                ui.label("clipboard");

                macro_rules! button {
                    ($name:ident, $text:literal, $callback:expr, $condition:expr $(, $kb:ident)?) => {
                        let mut $name = RichText::new($text);
                        if self.selected.is_empty() {
                            $name = $name.color(visuals.text_color().gamma_multiply(0.5));
                        }
                        let mut $name = Button::new($name)
                            .stroke(Stroke::NONE);
                        $(
                        $name = $name.shortcut_text(ctx.format_shortcut(&keybinds.$kb));
                        )?
                        if $condition {
                            $name = $name.sense(Sense::empty());
                        }
                        if ui.add($name).clicked() {
                            $callback;
                        }
                    };
                }

                macro_rules! selected_btn {
                    ($name:ident, $text:literal, $callback:expr $(, $kb:ident)?) => {
                        button!($name, $text, $callback, self.selected.is_empty()
                        $(
                        , $kb
                        )?
                        );
                    }
                }

                macro_rules! clipboard_btn {
                    ($name:ident, $text:literal, $callback:expr $(, $kb:ident)?) => {
                        button!($name, $text, $callback, self.clipboard.entries.is_empty()
                        $(
                        , $kb
                        )?
                        );
                    }
                }

                selected_btn!(
                    del,
                    "delete",
                    req_new_overlay = Some(OverlayKind::Delete),
                    delete_selections
                );
                selected_btn!(
                    cut,
                    "cut",
                    req_set_clipboard = Some(ClipboardMode::Cut),
                    cut_to_clipboard
                );
                selected_btn!(
                    copy,
                    "copy",
                    req_set_clipboard = Some(ClipboardMode::Copy),
                    copy_to_clipboard
                );
                selected_btn!(clear_s, "clear selection", req_reset_selected = true);
                clipboard_btn!(paste, "paste", req_paste = true, paste_from_clipboard);
                clipboard_btn!(clear_cp, "clear clipboard", req_reset_clipboard = true, clear_clipboard);

                ui.separator();
                ui.label("panels");

                if ui.add(Button::new("toggle clipboard")).clicked() {
                    req_toggle_panel = Some(PanelKind::Clipboard);
                }
            });

            // drag n drop handler
            if let (Some(from), Some(to)) = (from, to)
                && *from != to
            {
                req_transfer = Some(to);
            }
        });

        // modals

        let overlay = &self.overlay;
        let overlay_kind = overlay.kind;
        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Rename
        {
            let modal_widget = Modal::new(Id::new("rename_modal"));
            let mut content = overlay.buffer.clone();
            let error = &overlay.error.clone();

            modal_widget.show(&ctx, |ui| {
                ui.heading("renaming");
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    req_update_overlay_buffer = Some(content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        req_rename = true;
                    } else {
                        req_close_overlay = true;
                    }
                }

                input.request_focus();
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::CreateFile
        {
            let modal_widget = Modal::new(Id::new("create_file_modal"));
            let mut content = overlay.buffer.clone();
            let error = &overlay.error.clone();

            modal_widget.show(&ctx, |ui| {
                ui.label(format!("creating file at {}", self.current_path.display()));
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    req_update_overlay_buffer = Some(content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        req_create = Some(CreateType::File);
                    } else {
                        req_close_overlay = true;
                    }
                }

                input.request_focus();
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::CreateFolder
        {
            let modal_widget = Modal::new(Id::new("create_folder_modal"));
            let mut content = overlay.buffer.clone();
            let error = &overlay.error.clone();

            modal_widget.show(&ctx, |ui| {
                ui.label(format!(
                    "creating folder at {}",
                    self.current_path.display()
                ));
                let input = ui.add(TextEdit::singleline(&mut content));
                ui.add(Label::new(RichText::new(error).color(Color32::LIGHT_RED)));

                if input.changed() {
                    req_update_overlay_buffer = Some(content);
                }

                if input.lost_focus() {
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        req_create = Some(CreateType::Folder);
                    } else {
                        req_close_overlay = true;
                    }
                }

                input.request_focus();
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Paste
        {
            let modal_widget = Modal::new(Id::new("paste_modal"));
            modal_widget.show(&ctx, |ui| {
                let keybinds = &self.config.keybinds;
                ui.heading("duplicate found for:");
                let frame = Frame::NONE.fill(visuals.text_edit_bg_color());
                frame.show(ui, |f| {
                    f.label(format!("{}", overlay.path.as_ref().unwrap().display()));
                });
                ui.separator();
                ui.heading("choose pasting type");
                ui.vertical(|ui| {
                    if ui
                        .add(
                            Button::new("replace")
                                .shortcut_text(ctx.format_shortcut(&keybinds.choice_0)),
                        )
                        .clicked()
                    {
                        req_overlay_choice = Some(0);
                    }
                    ui.label("replace duplicated file");
                });
                ui.vertical(|ui| {
                    if ui
                        .add(
                            Button::new("duplicate")
                                .shortcut_text(ctx.format_shortcut(&keybinds.choice_1)),
                        )
                        .clicked()
                    {
                        req_overlay_choice = Some(1);
                    }
                    ui.label("make a new file with a number behind it");
                });

                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    req_close_overlay = true;
                }
            });
        };

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Delete
        {
            let keybinds = &self.config.keybinds;
            let modal_widget = Modal::new(Id::new("delete_modal"));
            let paths = self
                .selected
                .iter()
                .map(|entry_index| self.entries.children[*entry_index].path.as_ref());

            modal_widget.show(&ctx, |w| {
                w.label("are you sure you wanna delete these?");

                Frame::new()
                    .fill(visuals.text_edit_bg_color().gamma_multiply(0.7))
                    .corner_radius(4.0)
                    .inner_margin(2.0)
                    .show(w, |u| {
                        ScrollArea::vertical().max_height(200.0).show(u, |b| {
                            paths.for_each(|path: &Path| {
                                b.label(format!("{}", path.display()));
                            });
                        });
                    });

                w.separator();
                w.horizontal(|u| {
                    if u.add(
                        Button::new("yeah").shortcut_text(ctx.format_shortcut(&keybinds.choice_0)),
                    )
                    .clicked()
                    {
                        req_overlay_choice = Some(0);
                    }
                    if u.add(
                        Button::new("no").shortcut_text(ctx.format_shortcut(&keybinds.choice_1)),
                    )
                    .clicked()
                        || u.input(|i| i.key_pressed(Key::Escape))
                    {
                        req_overlay_choice = Some(1);
                    }
                })
            });
        }

        if let Some(kind) = overlay_kind
            && kind == OverlayKind::Metadata
        {
            let modal_widget = Modal::new(Id::new("metadata_modal"));
            let entry = self.overlay.entry.as_ref().unwrap();

            modal_widget.show(&ctx, |m| {
                m.label(format!("showing metadata for {}", entry.name));
                m.separator();

                self.config.view.metadata.iter().for_each(|p| match p {
                    Property::Path => {
                        m.label(format!("full path: {}", entry.path.display()));
                    }
                    Property::Type => {
                        m.label(format!("type: {}", entry.file_type));
                    }
                    Property::Accessed => {
                        m.label(format!(
                            "last accessed date: {}",
                            DateTime::from_timestamp_secs(entry.accessed.unwrap_or_default())
                                .unwrap()
                                .format(&self.config.view.format_date)
                        ));
                    }
                    Property::Created => {
                        m.label(format!(
                            "created date: {}",
                            DateTime::from_timestamp_secs(entry.created.unwrap_or_default())
                                .unwrap()
                                .format(&self.config.view.format_date)
                        ));
                    }
                    Property::Size => {
                        m.label(if let Some(size) = &entry.folder_size {
                            format!("folder size: {} items", size)
                        } else {
                            format!(
                                "file size: {}",
                                bytes_to_string(entry.file_size.unwrap_or_default())
                            )
                        });
                    }
                    _ => {}
                });
                if m.input(|i| i.key_pressed(Key::Escape)) {
                    req_close_overlay = true;
                }
            });
        }

        if req_close_overlay {
            self.overlay.reset();
        }
        if req_paste {
            self.paste();
        }
        if req_navigate_backward {
            self.nav_back();
        }
        if let Some(kind) = req_new_overlay {
            self.new_overlay(kind, None);
        }
        if req_reset_clipboard {
            self.clipboard.reset();
        }
        if req_reset_selected {
            self.clear_selected();
        }
        if let Some(mode) = req_set_clipboard {
            self.add_to_clipboard(mode);
        }
        if let Some(choice) = req_overlay_choice {
            self.finalize_overlay_choice(choice);
        }
        if let Some(index) = req_swap_selected {
            self.swap_selected(&index);
        }
        if let Some(content) = req_update_overlay_buffer {
            self.overlay.buffer(content);
        }
        if let Some(kind) = req_create {
            self.create(kind);
        }
        if let Some(index) = req_modify_selected {
            let ctrl_pressed =
                main_ui.input(|i| i.key_down(Key::ControlLeft) || i.key_down(Key::ControlRight));
            let shift_pressed =
                main_ui.input(|i| i.key_down(Key::ShiftLeft) || i.key_down(Key::ShiftRight));

            self.modify_selected(index, ctrl_pressed, shift_pressed);
        }
        if req_rename {
            self.rename();
        }
        if req_navigate_forward {
            self.nav_forward();
        }
        if req_unfocus_field {
            self.field.unfocus();
        }
        if let Some(kind) = req_open_field {
            self.new_field(kind);
        }
        if req_close_field {
            self.field.reset();
            self.filter_and_sort();
        }
        if let Some(content) = req_update_field_buffer {
            self.field.buffer(content);
        }
        if let Some(kind) = req_logic_field {
            self.logic_field(&kind);
        }
        if let Some(to) = req_transfer {
            self.transfer(to);
        }
        if let Some(kind) = req_toggle_panel {
            self.panels.toggle(kind);
        }

        let mut pending_close_panel = None;
        if self.panels.clipboard.is_some() {
            let mut panel_state = true;

            let clipboard = &self.clipboard;
            let window = Window::new("clipboard")
                .open(&mut panel_state)
                .enabled(true)
                .movable(true)
                .title_bar(true)
                .min_width(40.0)
                .min_height(100.0);

            window.show(&ctx, |win| {
                if clipboard.entries.is_empty() {
                    win.label("clipboard is empty!");
                    return;
                }
                win.label(clipboard.mode.as_ref().unwrap().clone());
                win.vertical(|v| {
                    let frame = Frame::NONE.fill(visuals.text_edit_bg_color());
                    frame.show(v, |f| {
                        clipboard.entries.iter().for_each(|p| {
                            f.label(p.to_str().unwrap_or_default());
                        });
                    });
                });
            });

            if !panel_state {
                pending_close_panel = Some(PanelKind::Clipboard);
            }
        }

        if let Some(kind) = pending_close_panel {
            self.panels.close(kind);
        }

        let toast_list = self.toasts.toasts.read();
        if !toast_list.is_empty() {
            let toast_overlay = Window::new("toast")
                .title_bar(false)
                .frame(Frame::NONE)
                .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-8.0, -8.0))
                .resizable(false);

            toast_overlay.show(main_ui, |overlay| {
                for toast in toast_list.iter() {
                    let mut frame = Frame::new()
                        .corner_radius(4.0)
                        .stroke(Stroke::new(1.0, Color32::TRANSPARENT))
                        .fill(visuals.text_edit_bg_color())
                        .inner_margin(4.0)
                        .outer_margin(4.0);

                    match toast.kind {
                        ToastKind::Info => frame.stroke.color = Color32::LIGHT_BLUE,
                        ToastKind::Danger => frame.stroke.color = Color32::LIGHT_RED,
                        ToastKind::Success => frame.stroke.color = Color32::LIGHT_GREEN,
                        ToastKind::Operation => frame.stroke.color = Color32::LIGHT_YELLOW,
                    }

                    frame.show(overlay, |fr| {
                        fr.vertical(|hor| {
                            hor.label(format!("{} (x{})", toast.title, toast.count));
                            hor.label(toast.content.clone());
                            if let Some(durr) = toast.duration {
                                hor.add(
                                    ProgressBar::new(
                                        Instant::now()
                                            .duration_since(toast.start_time)
                                            .div_duration_f32(durr),
                                    )
                                    .corner_radius(2.0)
                                    .desired_height(6.0)
                                    .fill(visuals.text_color().gamma_multiply(0.4)),
                                );
                            }
                            if let Some(per) = toast.percent {
                                hor.add(
                                    ProgressBar::new(per)
                                        .corner_radius(2.0)
                                        .desired_height(6.0)
                                        .fill(visuals.text_color().gamma_multiply(0.4)),
                                );
                            }
                        });
                    });
                }
            });
        }
    }
}

fn format_date(date: Option<i64>) -> String {
    let current_date = Utc::now();
    let given_date = DateTime::from_timestamp_secs(date.unwrap_or_default()).unwrap_or_default();
    let delta_day = current_date.sub(given_date).num_hours() / 24;

    // today
    if delta_day < 1 && current_date.day() == given_date.day() {
        format!("Today, {}", given_date.format("%I:%M %p"))
    }
    // yesterday
    else if delta_day < 2 {
        format!("Yesterday, {}", given_date.format("%I:%M %p"))
    }
    // this week
    else if delta_day <= 7 {
        format!("{} days ago", delta_day)
    }
    // last week
    else if delta_day <= 14 {
        String::from("Last week")
    }
    // this month
    else if delta_day <= 31 {
        format!("{} weeks ago", delta_day / 7)
    }
    // last month
    else if delta_day <= 62 {
        String::from("Last month")
    }
    // this year
    else if delta_day <= 365 {
        format!("{} months ago", delta_day / 31)
    }
    // last year
    else if delta_day <= 730 {
        String::from("Last year")
    }
    // blah blah blah
    else {
        format!("{} years ago", delta_day / 365)
    }
}

pub fn bytes_to_string(size: u64) -> String {
    // i dont think someone would have petabytes of data on their personal computer,,,
    if size >= 10_u64.pow(12) {
        // TiB
        let round = size / 10_u64.pow(12);
        format!("{:.2}TiB", round)
    } else if size >= 10_u64.pow(9) {
        // GiB
        let round = size / 10_u64.pow(9);
        format!("{:.2}GiB", round)
    } else if size >= 10_u64.pow(6) {
        // MiB
        let round = size / 10_u64.pow(6);
        format!("{:.2}MiB", round)
    } else if size >= 10_u64.pow(3) {
        // KiB
        let round = size / 10_u64.pow(3);
        format!("{:.2}KiB", round)
    } else {
        // bytes
        format!("{} bytes", size)
    }
}
