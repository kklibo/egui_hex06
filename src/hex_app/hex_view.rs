use crate::{
    hex_app::{ColorMode, HexApp, WhichFile},
    utilities::{byte_color, contrast, diff_at_index, diff_color},
};
use egui::{Context, RichText, TextStyle, Ui};

pub fn hex_view(hex_app: &mut HexApp, _ctx: &Context, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label("hex view");
        ui.checkbox(&mut hex_app.hex_view_color_mode, "colored text");
    });
    ui.separator();

    // Hopefully temporary code:
    // make the hex_view side panel automatically set its own width.
    ui.horizontal(|ui| {
        ui.set_invisible();

        let mut dummy_string = "00000000: ".to_string();
        (0..hex_app.hex_view_columns).for_each(|_| dummy_string += "00 ");

        ui.monospace(dummy_string);
    });

    if let Some(index) = hex_app.selected_index {
        ui.label(format!("selected index: 0x{:08X}", index));

        let data = match hex_app.active_file {
            WhichFile::File0 => &hex_app.pattern0,
            WhichFile::File1 => &hex_app.pattern1,
        };
        let other_data = match hex_app.active_file {
            WhichFile::File0 => &hex_app.pattern1,
            WhichFile::File1 => &hex_app.pattern0,
        };

        let columns_isize = isize::from(hex_app.hex_view_columns);
        let columns = usize::from(hex_app.hex_view_columns);
        if let Some(data) = data {
            if ui.ui_contains_pointer() {
                let scroll_delta = ui.input(|i| i.raw_scroll_delta);

                if scroll_delta.y != 0.0 {
                    let direction = -scroll_delta.y.signum() as isize;
                    let lines = 4;
                    let step = columns_isize * direction * lines;

                    let new_index = index
                        .saturating_add_signed(step)
                        .clamp(0, data.len().saturating_sub(1));
                    hex_app.selected_index = Some(new_index);
                }
            }

            if hex_app.hex_view_color_mode {
                for i in 0..hex_app.hex_view_rows {
                    let line_index = index + usize::from(i) * columns;
                    let address = format!("{:08X}:", line_index);
                    let mut display_text = String::new();
                    let mut offset = line_index;

                    ui.horizontal_wrapped(|ui| {
                        // Trick so we don't have to add spaces in the text below:
                        let width = ui.fonts(|f| {
                            f.glyph_width(&TextStyle::Monospace.resolve(ui.style()), ' ')
                        });
                        ui.spacing_mut().item_spacing.x = width - 0.5;
                        ui.label(
                            RichText::new(&address)
                                //.color(Color32::RED)
                                //.background_color(Color32::DARK_GRAY)
                                .monospace(),
                        );
                        while offset < data.len() && offset < line_index + columns {
                            let color = match hex_app.color_mode {
                                ColorMode::Value => byte_color(data[offset]),
                                ColorMode::Diff => {
                                    let diff_bytes =
                                        diff_at_index(&Some(data.as_ref()), other_data, offset);

                                    diff_color(diff_bytes, 1)
                                }
                            };

                            ui.label(
                                RichText::new(format!("{:02X}", data[offset]))
                                    .color(contrast(color))
                                    .background_color(color)
                                    .monospace(),
                            );
                            display_text += &format!("{:02X} ", data[offset]);
                            offset += 1;
                        }
                    });
                }
            } else {
                for i in 0..hex_app.hex_view_rows {
                    let line_index = index + usize::from(i) * columns;
                    let address = format!("{:08X}: ", line_index);
                    let mut display_text = String::new();
                    let mut offset = line_index;
                    while offset < data.len() && offset < line_index + columns {
                        display_text += &format!("{:02X} ", data[offset]);
                        offset += 1;
                    }

                    ui.monospace(address + &display_text);
                }
            }
        }
    } else {
        ui.label("no index selected");
    }
}
