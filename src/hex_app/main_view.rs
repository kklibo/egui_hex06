use std::collections::HashSet;

use crate::hex_app::{byte_color, byte_text, contrast, diff_color, ColorMode, HexApp, WhichFile};
use crate::range_blocks::{
    max_recursion_level, range_block_corners, Cacheable, CellCoords,
    CompleteLargestRangeBlockIterator, RangeBlockDiff, RangeBlockIterator, RangeBlockSum,
};
use crate::range_border::{LoopPairIter, LoopsIter, RangeBorder};
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

    *hex_app.rect_draw_count.borrow_mut() = 1;
    painter.rect_filled(painter.clip_rect(), 10.0, Color32::GRAY);

    let painter_coords = |point: CellCoords| -> Pos2 {
        let center = painter.clip_rect().center() + hex_app.pan;
        let coord = Pos2::new(point.x as f32, point.y as f32) * hex_app.zoom;
        coord + center.to_vec2()
    };
    let draw_rounded_corner =
        |start: CellCoords, corner: CellCoords, end: CellCoords, color: Color32| {
            let vec0 = painter_coords(corner) - painter_coords(start);
            let vec1 = painter_coords(end) - painter_coords(corner);

            let bound_size = (vec0 + vec1).abs();
            let clip_rect = Rect::from_center_size(painter_coords(corner), bound_size);

            let rect = Rect::from_two_pos(painter_coords(start), painter_coords(end));
            *hex_app.rect_draw_count.borrow_mut() += 1;
            painter.with_clip_rect(clip_rect).rect_stroke(
                rect.shrink(1.0),
                10.0,
                Stroke::new(2.0, color),
            );
        };

    let draw_rounded_filled_box =
        |top_left: CellCoords, bottom_right: CellCoords, color: Color32| {
            let rect = Rect::from_two_pos(painter_coords(top_left), painter_coords(bottom_right));
            *hex_app.rect_draw_count.borrow_mut() += 1;
            painter.rect_filled(rect, 10.0, color);
        };
    let draw_rounded_box = |top_left: CellCoords, bottom_right: CellCoords, color: Color32| {
        let rect = Rect::from_two_pos(painter_coords(top_left), painter_coords(bottom_right));
        *hex_app.rect_draw_count.borrow_mut() += 1;
        painter.rect_stroke(rect.shrink(1.0), 10.0, Stroke::new(2.0, color));
    };
    let draw_rounded_box1 = |top_left: CellCoords, bottom_right: CellCoords| {
        draw_rounded_box(top_left, bottom_right, Color32::GOLD);
    };
    let draw_rounded_box3 = |top_left: CellCoords, bottom_right: CellCoords| {
        draw_rounded_box(top_left, bottom_right, Color32::WHITE);
    };
    let draw_rounded_box4 = |top_left: CellCoords, bottom_right: CellCoords| {
        draw_rounded_box(top_left, bottom_right, Color32::BLACK);
    };
    let draw_point_circle = |point: CellCoords| {
        let coord = painter_coords(point);
        painter.circle_filled(coord, 2.0, Color32::GREEN);
    };
    let draw_cell_text =
        |top_left: CellCoords, bottom_right: CellCoords, color: Color32, text: &str| {
            let rect = Rect::from_two_pos(painter_coords(top_left), painter_coords(bottom_right));
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                text,
                FontId::monospace(hex_app.zoom * 0.75),
                color,
            );
        };
    let draw_centered_text =
        |top_left: CellCoords, bottom_right: CellCoords, color: Color32, text: &str| {
            let rect = Rect::from_two_pos(painter_coords(top_left), painter_coords(bottom_right));
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                text,
                FontId::default(),
                color,
            );
        };

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

        let is_visible = |index: u64, count: u64| {
            let (top_left, bottom_right) = range_block_corners(index, count, sub_block_sqrt);
            let rect = Rect::from_two_pos(painter_coords(top_left), painter_coords(bottom_right));

            painter.clip_rect().intersects(rect)
        };

        let visible_range_blocks_within = |target_recursion_level: u32, index: u64, count: u64| {
            RangeBlockIterator::new(
                index,
                index + count,
                target_recursion_level,
                max_recursion_level,
                sub_block_sqrt,
                is_visible,
            )
        };

        let selection_range_blocks = |index: u64, count: u64| {
            CompleteLargestRangeBlockIterator::new(
                index,
                index + count,
                max_recursion_level,
                sub_block_sqrt,
            )
        };

        let visible_range_blocks = |target_recursion_level: u32| {
            visible_range_blocks_within(target_recursion_level, 0, data_len)
        };

        if let Some(other_data) = other_data {
            let other_data_len: u64 = other_data
                .len()
                .try_into()
                .expect("other_data.len() should fit in u64");
            draw_range_border(
                selection_range_blocks(0, other_data_len),
                sub_block_sqrt,
                |start, corner, end| {
                    draw_rounded_corner(start, corner, end, Color32::DARK_GRAY);
                },
            );
        }

        for (index, count) in visible_range_blocks(rendered_recursion_level) {
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

            let (top_left, bottom_right) = range_block_corners(index, count, sub_block_sqrt);
            let rect = Rect::from_two_pos(painter_coords(top_left), painter_coords(bottom_right));

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

            if hex_app.ui_config.final_incomplete_block && index + count > data_len {
                // Final incomplete range block
                if let Some(count) = data_len.checked_sub(index) {
                    draw_range_boxes(
                        selection_range_blocks(index, count),
                        sub_block_sqrt,
                        |top_left, bottom_right| {
                            draw_rounded_filled_box(top_left, bottom_right, fill_color);
                        },
                    );
                    draw_range_border(
                        selection_range_blocks(index, count),
                        sub_block_sqrt,
                        |start, corner, end| {
                            draw_rounded_corner(start, corner, end, fill_color);
                        },
                    );
                } else {
                    // This should be impossible.
                    log::error!("index > data_len");
                }
                continue;
            }

            draw_rounded_filled_box(top_left, bottom_right, fill_color);

            let diff_text = if let Some(diff_bytes) = diff_bytes {
                format!("\n{}", diff_bytes as f32 / count as f32)
            } else {
                String::new()
            };

            if rendered_recursion_level == 0 {
                if hex_app.ui_config.cell_text {
                    let byte: u8 = data[usize::try_from(index).expect("temp fix")];
                    let display_text = byte_text(byte, hex_app.cell_view_mode);
                    draw_cell_text(top_left, bottom_right, contrast(fill_color), &display_text);
                }
            } else if hex_app.ui_config.block_address_text {
                let text = format!("0x{:08X}\n{} bytes\n{}", index, count, diff_text);
                draw_centered_text(top_left, bottom_right, contrast(fill_color), &text);
            }
        }

        if hex_app.ui_config.block_group_outline && rendered_recursion_level < max_recursion_level {
            for (index, count) in visible_range_blocks(rendered_recursion_level + 1) {
                let (top_left, bottom_right) = range_block_corners(index, count, sub_block_sqrt);
                draw_rounded_box4(top_left, bottom_right);
            }
        }

        if hex_app.ui_config.selected_subblock_boxes {
            if let Some(selected_index) = hex_app.selected_index {
                let selected_index = selected_index as u64;
                let mut search_index = 0;
                let mut search_count = data_len;

                for recursion_level in (0..rendered_recursion_level).rev() {
                    let contains_selected_index =
                        visible_range_blocks_within(recursion_level, search_index, search_count)
                            .find(|&(index, count)| {
                                index <= selected_index && selected_index < index + count
                            });

                    if let Some((index, count)) = contains_selected_index {
                        let (top_left, bottom_right) =
                            range_block_corners(index, count, sub_block_sqrt);
                        draw_rounded_filled_box(
                            top_left,
                            bottom_right,
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
        }

        if hex_app.ui_config.selected_block {
            for (index, count) in visible_range_blocks(rendered_recursion_level) {
                if let Some(selected_index) = hex_app.selected_index {
                    if index <= selected_index as u64 && (selected_index as u64) < index + count {
                        hex_app.selected_range_block = Some((index, count));

                        let (top_left, bottom_right) =
                            range_block_corners(index, count, sub_block_sqrt);
                        draw_rounded_box3(top_left, bottom_right);
                    }
                }
            }
        }

        if let Some(selected_index) = hex_app.selected_index {
            let count = u64::from(hex_app.hex_view_rows) * u64::from(hex_app.hex_view_columns);

            if hex_app.ui_config.selection_border_corner_points {
                draw_range_border_corners(
                    selection_range_blocks(selected_index as u64, count),
                    sub_block_sqrt,
                    draw_point_circle,
                );
            }

            if hex_app.ui_config.selection_boxes {
                draw_range_boxes(
                    selection_range_blocks(selected_index as u64, count),
                    sub_block_sqrt,
                    draw_rounded_box1,
                );
            }

            if hex_app.ui_config.selection_border {
                draw_range_border(
                    selection_range_blocks(selected_index as u64, count),
                    sub_block_sqrt,
                    |start, corner, end| {
                        draw_rounded_corner(start, corner, end, Color32::BLACK);
                    },
                );
            }
        }
    }

    if hex_app.ui_config.cursor {
        if let Some(cursor_pos) = response.hover_pos() {
            let rect = Rect::from_min_size(cursor_pos, Vec2::splat(10.0));
            *hex_app.rect_draw_count.borrow_mut() += 1;
            painter.rect_filled(rect, 0.0, byte_color(0));
        }
    }

    ui.expand_to_include_rect(painter.clip_rect());
}

fn draw_range_border(
    range_blocks: impl Iterator<Item = (u64, u64)>,
    sub_block_sqrt: u64,
    mut draw_corner: impl FnMut(CellCoords, CellCoords, CellCoords),
) {
    let mut range_border = RangeBorder::default();

    for (index, count) in range_blocks {
        let (top_left, bottom_right) = range_block_corners(index, count, sub_block_sqrt);
        range_border.add_rect(top_left, bottom_right);
    }

    let mut loops_iter = LoopsIter::new(range_border.edges);

    while let Some(loop_iter) = loops_iter.next() {
        for (edge, next_edge) in LoopPairIter::new(loop_iter) {
            assert_eq!(edge.end, next_edge.start);
            draw_corner(edge.start, edge.end, next_edge.end);
        }
    }
}

fn draw_range_boxes(
    range_blocks: impl Iterator<Item = (u64, u64)>,
    sub_block_sqrt: u64,
    mut draw_box: impl FnMut(CellCoords, CellCoords),
) {
    for (index, count) in range_blocks {
        let (top_left, bottom_right) = range_block_corners(index, count, sub_block_sqrt);
        draw_box(top_left, bottom_right);
    }
}

fn draw_range_border_corners(
    range_blocks: impl Iterator<Item = (u64, u64)>,
    sub_block_sqrt: u64,
    mut draw_point: impl FnMut(CellCoords),
) {
    let mut points = HashSet::new();

    for (index, count) in range_blocks {
        let (top_left, bottom_right) = range_block_corners(index, count, sub_block_sqrt);

        let top_right = CellCoords {
            x: bottom_right.x,
            y: top_left.y,
        };
        let bottom_left = CellCoords {
            x: top_left.x,
            y: bottom_right.y,
        };

        let vertices = [top_left, top_right, bottom_right, bottom_left];

        for vertex in vertices {
            if points.contains(&vertex) {
                points.remove(&vertex)
            } else {
                points.insert(vertex)
            };
        }
    }

    for point in points {
        draw_point(point);
    }
}
