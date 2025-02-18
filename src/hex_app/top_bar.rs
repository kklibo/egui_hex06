use egui::{Key, KeyboardShortcut, Modifiers};

use crate::hex_app::HexApp;

use super::{CellViewMode, ColorMode, WhichFile};

// Draws the control bar at the top of the window.
pub fn top_bar(hex_app: &mut HexApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    // Keyboard shortcuts for some of these controls.
    ctx.input_mut(|i| {
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F)) {
            hex_app.active_file = hex_app.active_file.next();
        }

        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::V)) {
            hex_app.cell_view_mode = hex_app.cell_view_mode.next();
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::C)) {
            hex_app.color_mode = hex_app.color_mode.next();
        }
    });

    ui.horizontal(|ui| {
        ui.heading("hex diff test (egui UI)");
        ui.separator();
        ui.selectable_value(&mut hex_app.active_file, WhichFile::File0, "File0");
        ui.selectable_value(&mut hex_app.active_file, WhichFile::File1, "File1");
        ui.separator();
        ui.label("zoom: ");
        ui.add(
            egui::DragValue::new(&mut hex_app.zoom)
                .speed(0.01)
                .range(HexApp::MIN_ZOOM..=HexApp::MAX_ZOOM),
        );
        ui.separator();

        ui.label("Cell View Mode:");
        ui.selectable_value(&mut hex_app.cell_view_mode, CellViewMode::Hex, "Hex");
        ui.selectable_value(&mut hex_app.cell_view_mode, CellViewMode::Ascii, "ASCII");

        ui.separator();

        ui.label("Color Mode:");
        ui.selectable_value(&mut hex_app.color_mode, ColorMode::Value, "Value");
        ui.selectable_value(&mut hex_app.color_mode, ColorMode::Diff, "Diff");
        ui.selectable_value(
            &mut hex_app.color_mode,
            ColorMode::Semantic01,
            "Semantic 01",
        );

        ui.separator();

        ui.label("Color Averaging:");
        ui.checkbox(&mut hex_app.color_averaging, "Color Averaging");
    });
}
