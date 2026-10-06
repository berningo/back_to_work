use eframe::egui;

use crate::model::{Action, DropPosition, Planner, Task, TaskState, count_tasks};

impl Planner {
    fn row(&mut self, ui: &mut egui::Ui, task: &Task, depth: usize, actions: &mut Vec<Action>, drop_target: &mut Option<(u64, DropPosition)>) {
        let id = task.id;
        let state = task.state();
        let leaf = task.children.is_empty();
        let title = task.title.clone();
        let children = task.children.clone();
        ui
            .horizontal(|ui| {
                ui.add_space(depth as f32 * 22.0);
                let (status_rect, status_response) =
                    ui.allocate_exact_size(egui::vec2(28.0, 30.0), egui::Sense::click());
                let painter = ui.painter();
                let center = status_rect.center();
                match state {
                    TaskState::Open => {
                        painter.circle_stroke(
                            center,
                            8.0,
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(130, 143, 162)),
                        );
                    }
                    TaskState::Partial => {
                        let amber = egui::Color32::from_rgb(222, 151, 61);
                        painter.circle_filled(center, 8.0, amber);
                        painter.line_segment(
                            [
                                center + egui::vec2(-4.0, 0.0),
                                center + egui::vec2(4.0, 0.0),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                        );
                    }
                    TaskState::Done => {
                        let green = egui::Color32::from_rgb(48, 164, 116);
                        painter.circle_filled(center, 8.0, green);
                        painter.line_segment(
                            [
                                center + egui::vec2(-4.0, 0.0),
                                center + egui::vec2(-1.0, 3.0),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                        );
                        painter.line_segment(
                            [
                                center + egui::vec2(-1.0, 3.0),
                                center + egui::vec2(4.0, -3.0),
                            ],
                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                        );
                    }
                }
                if status_response.clicked() && leaf {
                    actions.push(Action::Toggle(id));
                }
                let selected = self.selected == Some(id);
                let text_color = if state == TaskState::Done {
                    egui::Color32::from_rgb(132, 145, 163)
                } else {
                    egui::Color32::from_rgb(37, 50, 69)
                };
                let font = egui::FontId::proportional(14.0);
                let width = (ui.available_width() - 18.0).max(1.0);
                let galley = ui.painter().layout(title.clone(), font, text_color, width);
                let row_height = galley.size().y.max(30.0);
                let (rect, label) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), row_height),
                    egui::Sense::click_and_drag(),
                );
                if label.drag_started() { self.dragging = Some(id); }
                if self.dragging.is_some()
                    && let Some(pointer) = ui.ctx().pointer_latest_pos().filter(|p| rect.contains(*p))
                {
                        let edge = rect.height() * 0.2;
                        let position = if pointer.y < rect.top() + edge {
                            DropPosition::Before
                        } else if pointer.y > rect.bottom() - edge {
                            DropPosition::After
                        } else {
                            DropPosition::Inside
                        };
                        *drop_target = Some((id, position));
                        let blue = egui::Color32::from_rgb(54, 132, 235);
                        match position {
                            DropPosition::Inside => {
                                ui.painter().rect_stroke(rect, 5.0, egui::Stroke::new(2.0, blue), egui::StrokeKind::Inside);
                            }
                            DropPosition::Before | DropPosition::After => {
                                let line_y = if matches!(position, DropPosition::After) { rect.bottom() } else { rect.top() };
                                let x = rect.left() + depth as f32 * 22.0;
                                ui.painter().line_segment(
                                    [egui::pos2(x, line_y), egui::pos2(rect.right(), line_y)],
                                    egui::Stroke::new(2.0, blue),
                                );
                            }
                        }
                }
                if selected || label.hovered() {
                    ui.painter().rect_filled(
                        rect,
                        5.0,
                        if selected {
                            egui::Color32::from_rgb(224, 236, 253)
                        } else {
                            egui::Color32::from_rgb(239, 244, 250)
                        },
                    );
                }
                let text_top = rect.top() + (row_height - galley.size().y) * 0.5;
                ui.painter().galley(
                    rect.left_top() + egui::vec2(9.0, text_top - rect.top()),
                    galley.clone(),
                    text_color,
                );
                if state == TaskState::Done {
                    for line in &galley.rows {
                        let y = text_top + line.rect().center().y;
                        ui.painter().line_segment(
                            [
                                egui::pos2(rect.left() + 9.0, y),
                                egui::pos2(rect.left() + 9.0 + line.rect().width(), y),
                            ],
                            egui::Stroke::new(1.0, text_color),
                        );
                    }
                }
                if label.double_clicked() {
                    actions.push(Action::Rename(id));
                }
                if label.clicked() {
                    self.selected = Some(id);
                    self.persist();
                }
                label.context_menu(|ui| {
                    if ui.button("Aufgabe hinzufügen").clicked() {
                        actions.push(Action::AddRoot);
                        ui.close();
                    }
                    if ui.button("Unteraufgabe hinzufügen").clicked() {
                        actions.push(Action::AddChild(id));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Umbenennen  ·  F2").clicked() {
                        actions.push(Action::Rename(id));
                        ui.close();
                    }
                    if ui.button("Löschen  ·  Entf").clicked() {
                        actions.push(Action::Delete(id));
                        ui.close();
                    }
                    if leaf
                        && ui
                            .button(if state == TaskState::Done {
                                "Als offen markieren"
                            } else {
                                "Als erledigt markieren  ·  Leertaste"
                            })
                            .clicked()
                    {
                        actions.push(Action::Toggle(id));
                        ui.close();
                    }
                });
            });
        for child in &children {
            self.row(ui, child, depth + 1, actions, drop_target);
        }
    }
}

impl eframe::App for Planner {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgb(244, 247, 251).to_normalized_gamma_f32()
    }

    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = egui::Color32::from_rgb(244, 247, 251);
        visuals.window_fill = egui::Color32::WHITE;
        root_ui.ctx().set_visuals(visuals);

        let selected = self.selected;
        let is_leaf = selected
            .and_then(|id| Planner::find(&self.roots, id))
            .is_some_and(|task| task.children.is_empty());
        let mut actions = Vec::new();
        let mut drop_target = None;
        if self.dialog.is_none() {
            root_ui.ctx().input(|input| {
                if input.modifiers.command && input.key_pressed(egui::Key::N) {
                    actions.push(if input.modifiers.shift {
                        selected.map(Action::AddChild).unwrap_or(Action::AddRoot)
                    } else {
                        Action::AddRoot
                    });
                }
                if input.key_pressed(egui::Key::F2)
                    && let Some(id) = selected
                {
                    actions.push(Action::Rename(id));
                }
                if input.key_pressed(egui::Key::Delete)
                    && let Some(id) = selected
                {
                    actions.push(Action::Delete(id));
                }
                if input.key_pressed(egui::Key::Space) && is_leaf
                    && let Some(id) = selected
                {
                    actions.push(Action::Toggle(id));
                }
                if input.key_pressed(egui::Key::Escape) {
                    self.selected = None;
                    self.persist();
                }
            });
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(root_ui, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(244, 247, 251))
                    .inner_margin(egui::Margin::same(24))
                    .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                           egui::RichText::new("Oliver's")
                                .size(11.0)
                                .strong()
                                .color(egui::Color32::from_rgb(79, 112, 157)),
                        );
                        ui.add_space(4.0);
                        ui.heading(
                           egui::RichText::new("Aufgabenplaner")
                                .size(27.0)
                                .strong()
                                .color(egui::Color32::from_rgb(29, 43, 63)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+ Aufgabe").clicked() {
                            actions.push(Action::AddRoot);
                        }
                        if ui
                            .add_enabled(selected.is_some(), egui::Button::new("+ Unteraufgabe"))
                            .clicked()
                            && let Some(id) = selected
                        {
                            actions.push(Action::AddChild(id));
                        }
                    });
                });
                ui.add_space(20.0);

                let (total, done) = count_tasks(&self.roots);
                egui::Frame::new()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(225, 231, 240),
                    ))
                    .corner_radius(12)
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                               egui::RichText::new("MEINE AUFGABEN")
                                    .size(12.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(63, 80, 105)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                       egui::RichText::new(format!("{done} / {total} erledigt"))
                                            .color(egui::Color32::from_rgb(101, 117, 140)),
                                    );
                                },
                            );
                        });
                        ui.add_space(14.0);
                        if total > 0 {
                            ui.add(
                                egui::ProgressBar::new(done as f32 / total as f32)
                                    .desired_height(5.0)
                                    .fill(egui::Color32::from_rgb(71, 139, 230)),
                            );
                            ui.add_space(12.0);
                        }
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgb(248, 250, 253))
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgb(235, 239, 245),
                            ))
                            .corner_radius(9)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                if self.roots.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(24.0);
                                        ui.label(
                                           egui::RichText::new("Noch keine Aufgaben")
                                                .size(17.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(42, 57, 78)),
                                        );
                                        ui.label(
                                           egui::RichText::new(
                                                "Füge eine Aufgabe hinzu, um loszulegen.",
                                            )
                                            .color(egui::Color32::from_rgb(106, 122, 145)),
                                        );
                                        ui.add_space(24.0);
                                    });
                                }
                                let roots = self.roots.clone();
                                for task in &roots {
                                    self.row(ui, task, 0, &mut actions, &mut drop_target);
                                }
                            });
                    });
                ui.add_space(14.0);
                ui.label(
                    egui::RichText::new(if cfg!(target_os = "macos") {
                        "LEERTASTE  Erledigt     ·     Doppelklick / F2  Umbenennen     ·     Cmd+N  Neue Aufgabe     ·     Cmd+Umschalt+N  Unteraufgabe     ·     Entf  Löschen"
                    } else {
                        "LEERTASTE  Erledigt     ·     Doppelklick / F2  Umbenennen     ·     Ctrl+N  Neue Aufgabe     ·     Ctrl+Umschalt+N  Unteraufgabe     ·     Entf  Löschen"
                    })
                    .size(11.0)
                    .color(egui::Color32::from_rgb(111, 126, 147)),
                );
                    });
            });

        if root_ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary))
            && let Some(source) = self.dragging.take()
            && let Some((target, position)) = drop_target
            && source != target
        {
            actions.push(Action::Move(source, target, position));
        }

        for action in actions {
            match action {
                Action::AddRoot | Action::AddChild(_) | Action::Rename(_) => self.dialog(action),
                Action::Delete(id) => self.apply(Action::Delete(id), String::new()),
                Action::Toggle(id) => self.apply(Action::Toggle(id), String::new()),
                Action::Move(id, target, position) => self.move_task(id, target, position),
            }
        }

        let mut submit = None;
        let mut cancel = root_ui
            .ctx()
            .input(|input| input.key_pressed(egui::Key::Escape));
        if let Some(dialog) = &mut self.dialog {
            egui::Window::new(if matches!(dialog.action, Action::Rename(_)) {
                "Aufgabe umbenennen"
            } else {
                "Neue Aufgabe"
            })
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(root_ui.ctx(), |ui| {
                ui.label("Aufgabentitel:");
                let input = ui.add(
                    egui::TextEdit::singleline(&mut dialog.title)
                        .desired_width(300.0)
                        .hint_text("Titel eingeben"),
                );
                input.request_focus();
                let enter = (input.has_focus() || input.lost_focus())
                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Abbrechen").clicked() {
                        cancel = true;
                    }
                    if ui.button("Speichern").clicked() || enter {
                        let title = dialog.title.trim().to_owned();
                        if !title.is_empty() {
                            submit = Some((dialog.action.clone(), title));
                        }
                    }
                });
            });
        }
        if cancel {
            self.dialog = None;
        }
        if let Some((action, title)) = submit {
            self.apply(action, title);
            self.dialog = None;
        }
    }
}

