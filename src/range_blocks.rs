//! A *Range Block* is square graphical representation of a contiguous block of memory.
//! The smallest size of range block is called a *Cell*: it contains a single byte.
//! Larger range blocks encompass a square number (normally 16) of smaller ones:
//! one level of *recursion* is the representation of a range with one more such
//! encompassing step.
//!
//! A range block with a recursion level of 0 contains 1 cell.

use std::collections::HashMap;

/// Integer coordinate units for drawing cells and range blocks
/// in a two-dimensional rendering scheme. A cell is a single-byte block and has
/// a nominal size of 1x1, as measured in `CellCoords`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellCoords {
    pub x: u64,
    pub y: u64,
}

/// The `CellCoords` of the minimum (top-left) corner of the `index` byte's cell.
pub fn get_cell_offset(index: u64, sub_block_sqrt: u64) -> CellCoords {
    let sub_block_count = sub_block_sqrt * sub_block_sqrt;
    let (mut x, mut y) = (0, 0);
    let mut index = index;
    let mut scale = 1u64;

    while index > 0 {
        let sub_block_index = index % sub_block_count;

        x += (sub_block_index % sub_block_sqrt) * scale;
        y += (sub_block_index / sub_block_sqrt) * scale;

        index /= sub_block_count;
        scale *= sub_block_sqrt;
    }

    CellCoords { x, y }
}

/// Calculate the top-left and bottom-right corners of a range block.
/// Note: `index` and `count` should specify a real square range block,
/// otherwise the result may not be what you expect.
pub fn range_block_corners(
    index: u64,
    count: u64,
    sub_block_sqrt: u64,
) -> (CellCoords, CellCoords) {
    let top_left = get_cell_offset(index, sub_block_sqrt);

    let bottom_right = get_cell_offset(index + count - 1, sub_block_sqrt);
    let bottom_right = CellCoords {
        x: bottom_right.x + 1,
        y: bottom_right.y + 1,
    };

    (top_left, bottom_right)
}

/// The byte size of a range block at a recursion level.
pub fn range_block_size(recursion_level: u32, sub_block_sqrt: u64) -> u64 {
    sub_block_sqrt.pow(2 * recursion_level)
}

/// The maximum recursion level needed for this data length:
/// the lowest recursion level that contains at least `data_len` cells.
pub fn max_recursion_level(data_len: u64, sub_block_sqrt: u64) -> u32 {
    (data_len as f32)
        .log((sub_block_sqrt * sub_block_sqrt) as f32)
        .ceil() as u32
}

/// Finds the next range block at the target recursion level. Range blocks at or above
/// `target_recursion_level` that are encountered during the search will be tested with
/// `fn_filter`: if it returns `false`, the entire range block will be skipped.
///
/// This filter system is used to efficiently draw only the range blocks
/// that are currently visible in the UI view window.
pub fn next_range_block(
    search_start_index: u64,
    data_len: u64,
    target_recursion_level: u32,
    max_recursion_level: u32,
    sub_block_sqrt: u64,
    mut fn_filter: impl FnMut(u64, u64) -> bool,
) -> Option<(u64, u64)> {
    assert!(sub_block_sqrt > 1);

    let mut search_start_index = search_start_index;
    let mut max_recursion_level = max_recursion_level;
    let target_alignment = range_block_size(target_recursion_level, sub_block_sqrt);

    //Note:
    // "recursion level" refers to the number of times that sub-blocks are
    // grouped together into blocks.
    // Recursion level zero means no grouping: just individual bytes.

    loop {
        //assert!(search_start_index < data_len);
        assert!(target_recursion_level <= max_recursion_level);

        // Find the next index aligned with blocks at the target recursion level.
        let next_aligned_index = search_start_index.next_multiple_of(target_alignment);
        if next_aligned_index >= data_len {
            return None;
        }

        // Are there any higher recursion levels (larger blocks) that also align here?
        let max_aligned_recursion_level = (target_recursion_level..=max_recursion_level)
            .rev()
            .find(|i| 0 == next_aligned_index % range_block_size(*i, sub_block_sqrt))
            .expect("there should be an aligned recursion level");

        let max_aligned_recursion_level_block_size =
            range_block_size(max_aligned_recursion_level, sub_block_sqrt);

        // An aligned index is always at the start of at least one recursion block:
        //      Find the biggest one, and then
        //          return it (the target recursion level),
        //          recurse into it (bigger than our target),
        //          or skip it (filtered out)
        if fn_filter(next_aligned_index, max_aligned_recursion_level_block_size) {
            if max_aligned_recursion_level == target_recursion_level {
                return Some((next_aligned_index, max_aligned_recursion_level_block_size));
            }
            max_recursion_level -= 1;
        } else {
            search_start_index = next_aligned_index + max_aligned_recursion_level_block_size;
        }
    }
}

///`Iterator` over range blocks at the target recursion level.
/// Uses `next_range_block`: see it for more details.
pub struct RangeBlockIterator<'a> {
    search_start_index: u64,
    data_len: u64,
    target_recursion_level: u32,
    max_recursion_level: u32,
    sub_block_sqrt: u64,
    fn_filter: Box<dyn FnMut(u64, u64) -> bool + 'a>,
}

impl<'a> RangeBlockIterator<'a> {
    pub fn new(
        search_start_index: u64,
        data_len: u64,
        target_recursion_level: u32,
        max_recursion_level: u32,
        sub_block_sqrt: u64,
        fn_filter: impl FnMut(u64, u64) -> bool + 'a,
    ) -> Self {
        Self {
            search_start_index,
            data_len,
            target_recursion_level,
            max_recursion_level,
            sub_block_sqrt,
            fn_filter: Box::new(fn_filter),
        }
    }
}

impl Iterator for RangeBlockIterator<'_> {
    type Item = (u64, u64);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, count)) = next_range_block(
            self.search_start_index,
            self.data_len,
            self.target_recursion_level,
            self.max_recursion_level,
            self.sub_block_sqrt,
            &mut self.fn_filter,
        ) {
            self.search_start_index = index + count;
            Some((index, count))
        } else {
            None
        }
    }
}

/// Find the largest (highest recursion level) range block that
/// starts at `index` and ends before `limit_index`.
pub fn next_complete_largest_range_block(
    index: u64,
    limit_index: u64,
    max_recursion_level: u32,
    sub_block_sqrt: u64,
) -> Option<(u64, u64)> {
    assert!(sub_block_sqrt > 1);

    //Note:
    // "recursion level" refers to the number of times that sub-blocks are
    // grouped together into blocks.
    // Recursion level zero means no grouping: just individual bytes.

    // Find the largest range block that
    // - starts at index
    // - ends before limit_index

    (0..=max_recursion_level)
        .rev()
        .map(|i| range_block_size(i, sub_block_sqrt))
        .filter(|&size| 0 == index % size)
        .filter(|&size| index + size <= limit_index)
        .map(|size| (index, size))
        .next()
}

/// `Iterator` which produces the largest range blocks that fill a range.
/// Uses `next_complete_largest_range_block`: see it for more details.
pub struct CompleteLargestRangeBlockIterator {
    search_start_index: u64,
    search_end_index: u64,
    max_recursion_level: u32,
    sub_block_sqrt: u64,
}

impl CompleteLargestRangeBlockIterator {
    pub fn new(
        search_start_index: u64,
        search_end_index: u64,
        max_recursion_level: u32,
        sub_block_sqrt: u64,
    ) -> Self {
        Self {
            search_start_index,
            search_end_index,
            max_recursion_level,
            sub_block_sqrt,
        }
    }
}

impl Iterator for CompleteLargestRangeBlockIterator {
    type Item = (u64, u64);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((index, count)) = next_complete_largest_range_block(
            self.search_start_index,
            self.search_end_index,
            self.max_recursion_level,
            self.sub_block_sqrt,
        ) {
            self.search_start_index = index + count;
            Some((index, count))
        } else {
            None
        }
    }
}

/// `Cacheable` functions on range blocks can be stored in a `RangeBlockCache`.
pub trait Cacheable<T> {
    fn value(&self, index: u64, count: u64) -> T;
    fn value_from_sub_blocks(&self, value: &[T]) -> T;
}

/// `RangeBlockSum` is a `Cacheable` implementor that allows cached access to the sum
/// of bytes in a range block.
pub struct RangeBlockSum<'a> {
    data: &'a [u8],
}

impl<'a> RangeBlockSum<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn block_sum(&self, index: u64, count: u64) -> u64 {
        let limit =
            usize::try_from((self.data.len() as u64).min(index + count)).unwrap_or(usize::MAX);
        let index = usize::try_from(index).unwrap_or(usize::MAX);

        if index < self.data.len() {
            self.data[index..limit].iter().map(|&x| x as u64).sum()
        } else {
            0
        }
    }
}

impl Cacheable<u64> for RangeBlockSum<'_> {
    fn value(&self, index: u64, count: u64) -> u64 {
        self.block_sum(index, count)
    }

    fn value_from_sub_blocks(&self, value: &[u64]) -> u64 {
        value.iter().sum()
    }
}

/// `RangeBlockColorSum` is a `Cacheable` implementor that allows cached access to the sum
/// of the RGB color channels of every cell in a range block, according to some cell coloring scheme.
pub struct RangeBlockColorSum<'a, 'b> {
    data: &'a [u8],
    color_fn: Box<dyn Fn(u8) -> (u64, u64, u64) + 'b>,
}

impl<'a, 'b> RangeBlockColorSum<'a, 'b> {
    pub fn new(data: &'a [u8], color_fn: impl Fn(u8) -> (u64, u64, u64) + 'b) -> Self {
        Self {
            data,
            color_fn: Box::new(color_fn),
        }
    }

    pub fn block_color_sum(&self, index: u64, count: u64) -> (u64, u64, u64) {
        let limit =
            usize::try_from((self.data.len() as u64).min(index + count)).unwrap_or(usize::MAX);
        let index = usize::try_from(index).unwrap_or(usize::MAX);

        let (mut sum_r, mut sum_g, mut sum_b) = (0, 0, 0);

        if index < self.data.len() {
            for (r, g, b) in self.data[index..limit].iter().map(|&x| (self.color_fn)(x)) {
                sum_r += r;
                sum_g += g;
                sum_b += b;
            }
            (sum_r, sum_g, sum_b)
        } else {
            (0, 0, 0)
        }
    }
}

impl Cacheable<(u64, u64, u64)> for RangeBlockColorSum<'_, '_> {
    fn value(&self, index: u64, count: u64) -> (u64, u64, u64) {
        self.block_color_sum(index, count)
    }

    fn value_from_sub_blocks(&self, value: &[(u64, u64, u64)]) -> (u64, u64, u64) {
        value
            .iter()
            .fold((0, 0, 0), |(sum_r, sum_g, sum_b), (r, g, b)| {
                (sum_r + r, sum_g + g, sum_b + b)
            })
    }
}

/// `RangeBlockDiff` is a `Cacheable` implementor that allows cached access to the total count of
/// byte indices within a range block in which the index has different byte values between two files.
pub struct RangeBlockDiff<'a> {
    data0: &'a [u8],
    data1: &'a [u8],
}

impl<'a> RangeBlockDiff<'a> {
    pub fn new(data0: &'a [u8], data1: &'a [u8]) -> Self {
        Self { data0, data1 }
    }

    pub fn block_diff(&self, index: u64, count: u64) -> Option<usize> {
        let limit0 =
            usize::try_from((self.data0.len() as u64).min(index + count)).unwrap_or(usize::MAX);
        let limit1 =
            usize::try_from((self.data1.len() as u64).min(index + count)).unwrap_or(usize::MAX);
        let limit = std::cmp::min(limit0, limit1);
        let index = usize::try_from(index).unwrap_or(usize::MAX);
        let data_len = std::cmp::min(self.data0.len(), self.data1.len());

        if index < data_len {
            Some(
                self.data0[index..limit]
                    .iter()
                    .zip(self.data1[index..limit].iter())
                    .filter(|(a, b)| a != b)
                    .count(),
            )
        } else {
            None
        }
    }
}

impl Cacheable<Option<usize>> for RangeBlockDiff<'_> {
    fn value(&self, index: u64, count: u64) -> Option<usize> {
        self.block_diff(index, count)
    }

    fn value_from_sub_blocks(&self, value: &[Option<usize>]) -> Option<usize> {
        value.iter().flatten().copied().sum::<usize>().into()
    }
}

/// Uses `Cacheable` implementors to cache functions on range block contents.
/// This is used to provide fast lookup for
/// * the sum of byte values in a range block
/// * the byte difference count between the same range block in two loaded files
/// * and other things
pub struct RangeBlockCache<T: Clone> {
    values: HashMap<(u64, u64), T>,
}

impl<T: Clone> RangeBlockCache<T> {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get(&self, index: u64, count: u64) -> Option<T> {
        self.values.get(&(index, count)).cloned()
    }

    /// Generates a cache for `cacheable`. The lowest recursion levels are skipped to save storage space;
    /// they can be calculated quickly on demand.
    pub fn generate(cacheable: &impl Cacheable<T>, data_len: usize, sub_block_sqrt: u64) -> Self {
        let mut values = HashMap::new();
        let data_len: u64 = data_len.try_into().expect("data_len should fit in u64");
        let max_recursion_level = max_recursion_level(data_len, sub_block_sqrt);
        // Note: this works fine for sub_block_sqrt = 4; replace hardcode later?
        let min_recursion_level = 2;

        log::info!("max_recursion_level: {:?}", max_recursion_level);

        for i in min_recursion_level..=max_recursion_level {
            let mut cache_misses = 0;

            for (index, count) in
                RangeBlockIterator::new(0, data_len, i, i, sub_block_sqrt, |_, _| true)
            {
                if i <= min_recursion_level {
                    cache_misses += 1;
                    values.insert((index, count), cacheable.value(index, count));
                    continue;
                }

                let mut sub_accumulator = vec![];

                for (sub_index, sub_count) in RangeBlockIterator::new(
                    index,
                    index + count,
                    i - 1,
                    i - 1,
                    sub_block_sqrt,
                    |_, _| true,
                ) {
                    sub_accumulator.push(
                        values
                            .get(&(sub_index, sub_count))
                            .cloned()
                            .unwrap_or_else(|| {
                                cache_misses += 1;
                                cacheable.value(sub_index, sub_count)
                            }),
                    );
                }

                let value = cacheable.value_from_sub_blocks(&sub_accumulator);

                values.insert((index, count), value);
            }
            log::info!("values.len(): {:?}", values.len());
            log::info!("cache misses: {:?}", cache_misses);
        }

        log::info!("final values.len(): {:?}", values.len());

        Self { values }
    }
}
