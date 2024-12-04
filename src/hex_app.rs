use crate::{
    range_blocks::{Cacheable, RangeBlockCache, RangeBlockDiff, RangeBlockSum},
    utilities::{byte_color, contrast, diff_color},
};
use egui::{Vec2, Window};
use rand::Rng;

mod hex_view;
mod info_bar;
mod main_view;
mod top_bar;

#[derive(Debug, PartialEq)]
enum WhichFile {
    File0,
    File1,
}
#[derive(PartialEq)]
enum CellViewMode {
    Hex,
    Ascii,
}

#[derive(PartialEq)]
enum ColorMode {
    Value,
    Diff,
}

fn random_pattern(len: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    (0..len).map(|_| rng.gen_range(0..=255)).collect()
}

pub struct HexApp {
    source_name0: Option<String>,
    source_name1: Option<String>,
    pattern0: Option<Vec<u8>>,
    pattern1: Option<Vec<u8>>,
    cache0: RangeBlockCache<u64>,
    cache1: RangeBlockCache<u64>,
    diff_cache: RangeBlockCache<Option<usize>>,
    zoom: f32,
    pan: Vec2,
    active_file: WhichFile,
    dbg_notes: String,
    pan_velocity: Vec2,
    last_update_time: f64,
    hover_address: Option<usize>,
    cell_view_mode: CellViewMode,
    color_mode: ColorMode,
    hex_view_color_mode: bool,
    hex_view_columns: u8,
    hex_view_rows: u8,
    selected_index: Option<usize>,
    selected_range_block: Option<(u64, u64)>,
    rect_draw_count: usize,
}

impl HexApp {
    const MIN_ZOOM: f32 = 0.0025;
    const MAX_ZOOM: f32 = 128.0;
    const FRICTION: f32 = 0.9;
    pub const SUB_BLOCK_SQRT: u64 = 4;

    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let len = 10_000_usize;
        let mut data0 = random_pattern(len);
        data0.extend(0..=u8::MAX);
        let mut data1 = random_pattern(len);
        data1.extend(0..=u8::MAX);

        Self {
            source_name0: None,
            source_name1: None,
            cache0: RangeBlockCache::generate(&RangeBlockSum::new(&data0), data0.len(), 4),
            cache1: RangeBlockCache::generate(&RangeBlockSum::new(&data1), data1.len(), 4),
            diff_cache: RangeBlockCache::new(),
            pattern0: Some(data0),
            pattern1: Some(data1),
            zoom: 1.0,
            pan: Vec2::ZERO,
            active_file: WhichFile::File0,
            dbg_notes: String::new(),
            pan_velocity: Vec2::ZERO,
            last_update_time: 0.0,
            hover_address: None,
            cell_view_mode: CellViewMode::Hex,
            color_mode: ColorMode::Value,
            hex_view_color_mode: true,
            hex_view_columns: 16,
            hex_view_rows: 32,
            selected_index: None,
            selected_range_block: None,
            rect_draw_count: 0,
        }
    }
}

impl eframe::App for HexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
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
                                4,
                            );
                        }
                        WhichFile::File1 => {
                            log::info!("File1 dropped: {}", dropped_file.name);
                            self.pattern1 = Some(bytes.to_vec());
                            self.source_name1 = Some(dropped_file.name.clone());
                            self.cache1 = RangeBlockCache::generate(
                                &RangeBlockSum::new(self.pattern1.as_ref().unwrap()),
                                self.pattern1.as_ref().unwrap().len(),
                                4,
                            );
                        }
                    }
                    if let (Some(pattern0), Some(pattern1)) = (&self.pattern0, &self.pattern1) {
                        self.diff_cache = RangeBlockCache::generate(
                            &RangeBlockDiff::new(pattern0, pattern1),
                            std::cmp::max(pattern0.len(), pattern1.len()),
                            4,
                        );
                    }
                }
            }
        });

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
