use crate::hex_app::HexApp;

use super::{CellViewMode, ColorMode, WhichFile};

pub fn top_bar(hex_app: &mut HexApp, _ctx: &egui::Context, ui: &mut egui::Ui) {
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
    });
}
