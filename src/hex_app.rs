use crate::{
    range_blocks::{Cacheable, RangeBlockCache, RangeBlockColorSum, RangeBlockDiff, RangeBlockSum},
    utilities::{byte_color_rgb, semantic01_color_rgb},
};
use egui::{Vec2, Window};
use rand::Rng;
use std::cell::RefCell;
mod frame_history;
mod hex_view;
mod info_bar;
mod main_view;
mod top_bar;

#[derive(Debug, PartialEq)]
enum WhichFile {
    File0,
    File1,
}

impl WhichFile {
    pub fn next(&self) -> Self {
        match self {
            WhichFile::File0 => WhichFile::File1,
            WhichFile::File1 => WhichFile::File0,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum CellViewMode {
    Hex,
    Ascii,
}

impl CellViewMode {
    pub fn next(&self) -> Self {
        match self {
            CellViewMode::Hex => CellViewMode::Ascii,
            CellViewMode::Ascii => CellViewMode::Hex,
        }
    }
}

fn byte_text(byte: u8, cell_view_mode: CellViewMode) -> String {
    match cell_view_mode {
        CellViewMode::Hex => format!("{byte:02X}"),
        CellViewMode::Ascii => if byte.is_ascii_graphic() {
            byte as char
        } else {
            '.'
        }
        .to_string(),
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum ColorMode {
    Value,
    Diff,
    Semantic01,
}

impl ColorMode {
    pub fn next(&self) -> Self {
        match self {
            ColorMode::Value => ColorMode::Diff,
            ColorMode::Diff => ColorMode::Semantic01,
            ColorMode::Semantic01 => ColorMode::Value,
        }
    }
}

fn random_pattern(len: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..len).map(|_| rng.gen_range(0..=255)).collect()
}

pub struct UIConfig {
    pub final_incomplete_block: bool,
    pub cell_text: bool,
    pub block_address_text: bool,
    pub block_group_outline: bool,
    pub selection_boxes: bool,
    pub selection_border_corner_points: bool,
    pub selection_border: bool,
    pub selected_subblock_boxes: bool,
    pub selected_block: bool,
    pub cursor: bool,
}

pub struct HexApp {
    source_name0: Option<String>,
    source_name1: Option<String>,
    pattern0: Option<Vec<u8>>,
    pattern1: Option<Vec<u8>>,
    cache0: RangeBlockCache<u64>,
    cache1: RangeBlockCache<u64>,
    diff_cache: RangeBlockCache<Option<usize>>,
    color_cache_value0: RangeBlockCache<(u64, u64, u64)>,
    color_cache_value1: RangeBlockCache<(u64, u64, u64)>,
    color_cache_semantic01_0: RangeBlockCache<(u64, u64, u64)>,
    color_cache_semantic01_1: RangeBlockCache<(u64, u64, u64)>,
    zoom: f32,
    pan: Vec2,
    active_file: WhichFile,
    dbg_notes: String,
    dbg_flag: bool,
    pan_velocity: Vec2,
    last_update_time: f64,
    hover_address: Option<usize>,
    cell_view_mode: CellViewMode,
    color_mode: ColorMode,
    color_averaging: bool,
    hex_view_color_mode: bool,
    hex_view_columns: u8,
    hex_view_rows: u8,
    selected_index: Option<usize>,
    selected_range_block: Option<(u64, u64)>,
    rect_draw_count: RefCell<usize>,
    ui_config_window: bool,
    ui_config: UIConfig,
    frame_history: frame_history::FrameHistory,
}

impl HexApp {
    const MIN_ZOOM: f32 = 0.0025;
    const MAX_ZOOM: f32 = 128.0;
    const FRICTION: f32 = 0.9;
    pub const SUB_BLOCK_SQRT: u64 = 4;

    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let len0 = 10_000_usize;
        let mut data0 = random_pattern(len0);
        data0.extend(0..=u8::MAX);
        let len1 = 12_000_usize;
        let mut data1 = random_pattern(len1);
        data1.extend(0..=u8::MAX);

        Self {
            source_name0: None,
            source_name1: None,
            cache0: RangeBlockCache::generate(
                &RangeBlockSum::new(&data0),
                data0.len(),
                Self::SUB_BLOCK_SQRT,
            ),
            cache1: RangeBlockCache::generate(
                &RangeBlockSum::new(&data1),
                data1.len(),
                Self::SUB_BLOCK_SQRT,
            ),
            diff_cache: RangeBlockCache::new(),
            color_cache_value0: RangeBlockCache::generate(
                &RangeBlockColorSum::new(&data0, byte_color_rgb),
                data0.len(),
                Self::SUB_BLOCK_SQRT,
            ),
            color_cache_value1: RangeBlockCache::generate(
                &RangeBlockColorSum::new(&data1, byte_color_rgb),
                data1.len(),
                Self::SUB_BLOCK_SQRT,
            ),
            color_cache_semantic01_0: RangeBlockCache::generate(
                &RangeBlockColorSum::new(&data0, semantic01_color_rgb),
                data0.len(),
                Self::SUB_BLOCK_SQRT,
            ),
            color_cache_semantic01_1: RangeBlockCache::generate(
                &RangeBlockColorSum::new(&data1, semantic01_color_rgb),
                data1.len(),
                Self::SUB_BLOCK_SQRT,
            ),
            pattern0: Some(data0),
            pattern1: Some(data1),
            zoom: 1.0,
            pan: Vec2::ZERO,
            active_file: WhichFile::File0,
            dbg_notes: String::new(),
            dbg_flag: false,
            pan_velocity: Vec2::ZERO,
            last_update_time: 0.0,
            hover_address: None,
            cell_view_mode: CellViewMode::Hex,
            color_mode: ColorMode::Value,
            color_averaging: true,
            hex_view_color_mode: true,
            hex_view_columns: 16,
            hex_view_rows: 32,
            selected_index: None,
            selected_range_block: None,
            rect_draw_count: RefCell::new(0),
            ui_config_window: false,
            ui_config: UIConfig {
                final_incomplete_block: true,
                cell_text: true,
                block_address_text: true,
                block_group_outline: true,
                selection_boxes: true,
                selection_border_corner_points: true,
                selection_border: true,
                selected_subblock_boxes: true,
                selected_block: true,
                cursor: true,
            },
            frame_history: frame_history::FrameHistory::default(),
        }
    }
}

impl eframe::App for HexApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);

        ctx.input(|i| {
            // Handle files dropped into the window: load the file and update the caches.
            if let Some(dropped_file) = i.raw.dropped_files.first() {
                if let Some(bytes) = &dropped_file.bytes {
                    match self.active_file {
                        WhichFile::File0 => {
                            log::info!("File0 dropped: {}", dropped_file.name);
                            self.pattern0 = Some(bytes.to_vec());
                            self.source_name0 = Some(dropped_file.name.clone());
                            self.cache0 = RangeBlockCache::generate(
                                &RangeBlockSum::new(self.pattern0.as_ref().unwrap()),
                                self.pattern0.as_ref().unwrap().len(),
                                Self::SUB_BLOCK_SQRT,
                            );
                            self.color_cache_value0 = RangeBlockCache::generate(
                                &RangeBlockColorSum::new(
                                    self.pattern0.as_ref().unwrap(),
                                    byte_color_rgb,
                                ),
                                self.pattern0.as_ref().unwrap().len(),
                                Self::SUB_BLOCK_SQRT,
                            );
                            self.color_cache_semantic01_0 = RangeBlockCache::generate(
                                &RangeBlockColorSum::new(
                                    self.pattern0.as_ref().unwrap(),
                                    semantic01_color_rgb,
                                ),
                                self.pattern0.as_ref().unwrap().len(),
                                Self::SUB_BLOCK_SQRT,
                            );
                        }
                        WhichFile::File1 => {
                            log::info!("File1 dropped: {}", dropped_file.name);
                            self.pattern1 = Some(bytes.to_vec());
                            self.source_name1 = Some(dropped_file.name.clone());
                            self.cache1 = RangeBlockCache::generate(
                                &RangeBlockSum::new(self.pattern1.as_ref().unwrap()),
                                self.pattern1.as_ref().unwrap().len(),
                                Self::SUB_BLOCK_SQRT,
                            );
                            self.color_cache_value1 = RangeBlockCache::generate(
                                &RangeBlockColorSum::new(
                                    self.pattern1.as_ref().unwrap(),
                                    byte_color_rgb,
                                ),
                                self.pattern1.as_ref().unwrap().len(),
                                Self::SUB_BLOCK_SQRT,
                            );
                            self.color_cache_semantic01_1 = RangeBlockCache::generate(
                                &RangeBlockColorSum::new(
                                    self.pattern1.as_ref().unwrap(),
                                    semantic01_color_rgb,
                                ),
                                self.pattern1.as_ref().unwrap().len(),
                                Self::SUB_BLOCK_SQRT,
                            );
                        }
                    }
                    if let (Some(pattern0), Some(pattern1)) = (&self.pattern0, &self.pattern1) {
                        self.diff_cache = RangeBlockCache::generate(
                            &RangeBlockDiff::new(pattern0, pattern1),
                            std::cmp::max(pattern0.len(), pattern1.len()),
                            Self::SUB_BLOCK_SQRT,
                        );
                    }
                }
            }
        });

        // UI config options window (opened via bottom bar button).
        Window::new("UI Config")
            .open(&mut self.ui_config_window)
            .show(ctx, |ui| {
                ui.checkbox(
                    &mut self.ui_config.final_incomplete_block,
                    "Final incomplete block",
                );
                ui.checkbox(&mut self.ui_config.cell_text, "Cell text");
                ui.checkbox(&mut self.ui_config.block_address_text, "Block address text");
                ui.checkbox(
                    &mut self.ui_config.block_group_outline,
                    "Block group outline",
                );
                ui.checkbox(&mut self.ui_config.selection_boxes, "Selection boxes");
                ui.checkbox(
                    &mut self.ui_config.selection_border_corner_points,
                    "Selection border corner points",
                );
                ui.checkbox(&mut self.ui_config.selection_border, "Selection border");
                ui.checkbox(
                    &mut self.ui_config.selected_subblock_boxes,
                    "Selected subblock boxes",
                );
                ui.checkbox(&mut self.ui_config.selected_block, "Selected block");
                ui.checkbox(&mut self.ui_config.cursor, "Cursor");
            });

        // Info window for highlighted range block at the current visible recursion level.
        // (may want to replace this entire concept)
        Window::new("Block info").show(ctx, |ui| {
            if let Some((index, count)) = self.selected_range_block {
                ui.label(format!(
                    "Selected range block: 0x{index:08X}; size: {count} bytes"
                ));
                if let Some(data) = &self.pattern0 {
                    let sum0 = self
                        .cache0
                        .get(index, count)
                        .unwrap_or_else(|| RangeBlockSum::new(data).value(index, count));
                    let average0 = sum0 as f32 / count as f32;

                    ui.label(format!("File0 Average byte value: {}", average0));
                }
                if let Some(data) = &self.pattern1 {
                    let sum1 = self
                        .cache1
                        .get(index, count)
                        .unwrap_or_else(|| RangeBlockSum::new(data).value(index, count));
                    let average1 = sum1 as f32 / count as f32;
                    ui.label(format!("File1 Average byte value: {}", average1));
                }

                if let (Some(data0), Some(data1)) = (&self.pattern0, &self.pattern1) {
                    let diff = self
                        .diff_cache
                        .get(index, count)
                        .unwrap_or_else(|| RangeBlockDiff::new(data0, data1).value(index, count));

                    if let Some(diff) = diff {
                        ui.label(format!(
                            "Diff: {} bytes ({}%)",
                            diff,
                            100.0 * diff as f32 / count as f32
                        ));
                    }
                }
            }
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            top_bar::top_bar(self, ctx, ui);
        });

        egui::TopBottomPanel::bottom("bottom panel").show(ctx, |ui| {
            crate::hex_app::info_bar::info_bar(self, ui);
        });

        egui::SidePanel::left("left panel").show(ctx, |ui| {
            hex_view::hex_view(self, ctx, ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            main_view::main_view(self, ctx, ui);
        });
    }
}
