use egui::Ui;

pub fn info_bar(hex_app: &mut crate::hex_app::HexApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.checkbox(&mut hex_app.dbg_flag, "dbg_flag");
        ui.label(format!("self.zoom: {}", hex_app.zoom));
        ui.label(format!("self.pan: {:?}", hex_app.pan));
        ui.label(format!("pan_velocity: {:?}", hex_app.pan_velocity));
        ui.separator();
        if let Some(address) = hex_app.hover_address {
            ui.label(format!("Address: 0x{:08X}", address));
        } else {
            ui.label("Address: N/A");
        }
        ui.separator();
        ui.label(format!("dbg: {}", hex_app.dbg_notes));
        ui.label(format!("rect_draw_count: {}", hex_app.rect_draw_count));
    });
}
