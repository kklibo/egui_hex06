use std::collections::{BTreeSet, HashSet};

use crate::hex_app::{
    byte_color, contrast, diff_color, CellViewMode, ColorMode, HexApp, WhichFile,
};
use crate::range_blocks::{
    get_cell_offset, max_recursion_level, range_block_rect, Cacheable,
    CompleteLargestRangeBlockIterator, RangeBlockDiff, RangeBlockIterator, RangeBlockSum,
};
use egui::{Align2, Color32, Context, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub fn main_view(hex_app: &mut HexApp, _ctx: &Context, ui: &mut Ui) {
    hex_app.selected_range_block = None; // Reset selected range block (should this be done some other way?)

    let (response, painter) =
        ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

    if ui.ui_contains_pointer() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);

        if scroll_delta.y != 0.0 {
            let prev_zoom = hex_app.zoom;
            hex_app.zoom *= 1.0 + scroll_delta.y * 0.005;

            // Clamp zoom to intended range to prevent smooth scroll "bounce" effect.
            hex_app.zoom = hex_app.zoom.clamp(HexApp::MIN_ZOOM, HexApp::MAX_ZOOM);

            // Add a pan offset to keep the zoom effect centered on the mouse cursor.
            if let Some(cursor_pos) = response.hover_pos() {
                let screen_center_to_cursor = cursor_pos - painter.clip_rect().center();
                let pan_center_to_cursor = screen_center_to_cursor - hex_app.pan;
                hex_app.pan -= pan_center_to_cursor * ((hex_app.zoom / prev_zoom) - 1.0);
            }
        }
    }

    let current_time = ui.input(|i| i.time);
    let dt = (current_time - hex_app.last_update_time) as f32;
    hex_app.last_update_time = current_time;

    if response.dragged() {
        hex_app.pan_velocity = response.drag_delta() / dt;
    } else {
        hex_app.pan += hex_app.pan_velocity * dt;
        hex_app.pan_velocity *= HexApp::FRICTION.powf(dt * 60.0);
    }

    hex_app.pan += response.drag_delta();

    hex_app.rect_draw_count = 1;
    painter.rect_filled(painter.clip_rect(), 10.0, Color32::GRAY);

    let center = painter.clip_rect().center() + hex_app.pan;

    let data = match hex_app.active_file {
        WhichFile::File0 => &hex_app.pattern0,
        WhichFile::File1 => &hex_app.pattern1,
    };
    let other_data = match hex_app.active_file {
        WhichFile::File0 => &hex_app.pattern1,
        WhichFile::File1 => &hex_app.pattern0,
    };

    let data_cache = match hex_app.active_file {
        WhichFile::File0 => &hex_app.cache0,
        WhichFile::File1 => &hex_app.cache1,
    };

    if let Some(data) = data {
        let data_len: u64 = data.len().try_into().expect("data.len() should fit in u64");
        let sub_block_sqrt = HexApp::SUB_BLOCK_SQRT;
        let max_recursion_level = max_recursion_level(data_len, sub_block_sqrt);
        let rendered_recursion_level = std::cmp::min(max_recursion_level, {
            let cell_width = painter.clip_rect().width() / hex_app.zoom;

            cell_width.log(sub_block_sqrt as f32) as u32 - 1
        });

        hex_app.dbg_notes = format!(
            "max_recursion_level: {}, rendered_recursion_level: {}",
            max_recursion_level, rendered_recursion_level
        );

        let visible_range_blocks_within = |target_recursion_level: u32, index: u64, count: u64| {
            let fn_filter = |index: u64, count: u64| -> bool {
                let rect = range_block_rect(index, count, sub_block_sqrt, hex_app.zoom);
                let rect = rect.translate(center.to_vec2());

                painter.clip_rect().intersects(rect)
            };

            let range_block_iterator = RangeBlockIterator::new(
                index,
                index + count,
                target_recursion_level,
                max_recursion_level,
                sub_block_sqrt,
                fn_filter,
            );

            range_block_iterator.map(|(index, count)| {
                let rect = range_block_rect(index, count, sub_block_sqrt, hex_app.zoom);
                let rect = rect.translate(center.to_vec2());
                (index, count, rect)
            })
        };

        let selection_range_blocks = |index: u64, count: u64| {
            let fn_filter = |index: u64, count: u64| -> bool {
                let rect = range_block_rect(index, count, sub_block_sqrt, hex_app.zoom);
                let rect = rect.translate(center.to_vec2());

                painter.clip_rect().intersects(rect)
            };

            let range_block_iterator = CompleteLargestRangeBlockIterator::new(
                index,
                index + count,
                max_recursion_level,
                sub_block_sqrt,
            );

            range_block_iterator
                .map(|(index, count)| {
                    let rect = range_block_rect(index, count, sub_block_sqrt, hex_app.zoom);
                    let rect = rect.translate(center.to_vec2());
                    (index, count, rect)
                })
                .filter(move |&(index, count, _rect)| fn_filter(index, count))
        };

        let visible_range_blocks = |target_recursion_level: u32| {
            visible_range_blocks_within(target_recursion_level, 0, data_len)
        };

        for (index, count, rect) in visible_range_blocks(rendered_recursion_level) {
            if index + count > data_len {
                // Final incomplete range block
                if let Some(count) = data_len.checked_sub(index) {
                    for (_index, _count, rect) in selection_range_blocks(index, count) {
                        hex_app.rect_draw_count += 1;
                        painter.rect_stroke(
                            rect.shrink(1.0),
                            10.0,
                            Stroke::new(2.0, Color32::DARK_RED),
                        );
                    }
                } else {
                    // This should be impossible.
                    log::error!("index > data_len");
                }
                continue;
            }

            let diff_bytes = if hex_app.color_mode == ColorMode::Diff {
                if let Some(other_data) = other_data {
                    hex_app.diff_cache.get(index, count).unwrap_or_else(|| {
                        RangeBlockDiff::new(data, other_data).value(index, count)
                    })
                } else {
                    None
                }
            } else {
                None
            };

            let fill_color = if response.clicked()
                && response
                    .interact_pointer_pos()
                    .map(|pos| rect.contains(pos))
                    .unwrap_or(false)
            {
                hex_app.selected_index = Some(index.try_into().expect("temp fix"));
                Color32::WHITE
            } else {
                match hex_app.color_mode {
                    ColorMode::Value => {
                        let sum = data_cache
                            .get(index, count)
                            .unwrap_or_else(|| RangeBlockSum::new(data).value(index, count));
                        let average = sum as f32 / count as f32;
                        byte_color(average as u8)
                    }
                    ColorMode::Diff => diff_color(diff_bytes, count),
                }
            };

            hex_app.rect_draw_count += 1;
            painter.rect_filled(rect, 10.0, fill_color);

            let diff_text = if let Some(diff_bytes) = diff_bytes {
                format!("\n{}", diff_bytes as f32 / count as f32)
            } else {
                String::new()
            };

            if rendered_recursion_level == 0 {
                let byte: u8 = data[usize::try_from(index).expect("temp fix")];
                let display_text = match hex_app.cell_view_mode {
                    CellViewMode::Hex => format!("{byte:02X}"),
                    CellViewMode::Ascii => if byte.is_ascii_graphic() {
                        byte as char
                    } else {
                        '.'
                    }
                    .to_string(),
                };

                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    display_text,
                    FontId::monospace(hex_app.zoom * 0.75),
                    contrast(fill_color),
                );
            } else {
                let text = format!("0x{:08X}\n{} bytes\n{}", index, count, diff_text);
                let text_pos = rect.center();
                painter.text(
                    text_pos,
                    Align2::CENTER_CENTER,
                    text,
                    FontId::default(),
                    contrast(fill_color),
                );
            }
        }

        if rendered_recursion_level < max_recursion_level {
            for (_index, _count, rect) in visible_range_blocks(rendered_recursion_level + 1) {
                hex_app.rect_draw_count += 1;
                painter.rect_stroke(rect.shrink(1.0), 10.0, Stroke::new(2.0, Color32::BLACK));
            }
        }

        if let Some(selected_index) = hex_app.selected_index {
            let selected_index = selected_index as u64;
            let mut search_index = 0;
            let mut search_count = data_len;

            for recursion_level in (0..rendered_recursion_level).rev() {
                let contains_selected_index =
                    visible_range_blocks_within(recursion_level, search_index, search_count).find(
                        |&(index, count, _rect)| {
                            index <= selected_index && selected_index < index + count
                        },
                    );

                if let Some((index, count, rect)) = contains_selected_index {
                    hex_app.rect_draw_count += 1;
                    painter.rect_filled(
                        rect,
                        10.0,
                        Color32::from_rgba_unmultiplied(128, 128, 128, 192),
                    );
                    // Constrain search to this range block.
                    search_index = index;
                    search_count = count;
                } else {
                    // Selected index is off-screen.
                    break;
                }
            }
        }

        for (index, count, rect) in visible_range_blocks(rendered_recursion_level) {
            if let Some(selected_index) = hex_app.selected_index {
                if index <= selected_index as u64 && (selected_index as u64) < index + count {
                    hex_app.selected_range_block = Some((index, count));
                    hex_app.rect_draw_count += 1;
                    painter.rect_stroke(rect.shrink(1.0), 10.0, Stroke::new(2.0, Color32::WHITE));
                }
            }
        }

        if let Some(selected_index) = hex_app.selected_index {
            let mut points = HashSet::new();

            let mut include_block = |index: u64, count: u64| {
                let (x_min, y_min) = get_cell_offset(index, sub_block_sqrt);
                let (x_max, y_max) = get_cell_offset(index + count - 1, sub_block_sqrt);
                let x_max = x_max + 1;
                let y_max = y_max + 1;

                let vertices = [
                    (x_min, y_min),
                    (x_max, y_min),
                    (x_max, y_max),
                    (x_min, y_max),
                ];

                for vertex in vertices {
                    if points.contains(&vertex) {
                        points.remove(&vertex)
                    } else {
                        points.insert(vertex)
                    };
                }
            };

            for (index, count, rect) in selection_range_blocks(
                selected_index as u64,
                u64::from(hex_app.hex_view_rows) * u64::from(hex_app.hex_view_columns),
            ) {
                include_block(index, count);

                hex_app.rect_draw_count += 1;
                painter.rect_stroke(rect.shrink(1.0), 10.0, Stroke::new(2.0, Color32::GOLD));
            }

            for point in points {
                hex_app.rect_draw_count += 1;

                let coord = Pos2::new(point.0 as f32, point.1 as f32) * hex_app.zoom;
                let coord = coord + center.to_vec2();
                //let rect = range_block_rect(index, count, sub_block_sqrt, hex_app.zoom);
                //let rect = rect.translate(center.to_vec2());

                painter.circle_filled(coord, 2.0, Color32::GREEN);
            }
        }
    }

    if let Some(cursor_pos) = response.hover_pos() {
        let rect = Rect::from_min_size(cursor_pos, Vec2::splat(10.0));
        hex_app.rect_draw_count += 1;
        painter.rect_filled(rect, 0.0, byte_color(0));
    }

    ui.expand_to_include_rect(painter.clip_rect());
}
