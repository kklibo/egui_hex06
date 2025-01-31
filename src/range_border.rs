use std::collections::HashMap;

use crate::range_blocks::CellCoords;

#[derive(Default)]
pub struct RangeBorder {
    next_edge_id: usize,
    pub edges: Vec<Edge>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Edge {
    pub id: usize,
    pub next: usize,
    pub start: CellCoords,
    pub end: CellCoords,
}

impl RangeBorder {
    pub fn add_rect(&mut self, top_left: CellCoords, bottom_right: CellCoords) {
        let top_right = CellCoords {
            x: bottom_right.x,
            y: top_left.y,
        };
        let bottom_left = CellCoords {
            x: top_left.x,
            y: bottom_right.y,
        };

        let id = self.next_edge_id;
        self.add_edge(id, id + 1, top_left, top_right);
        self.add_edge(id + 1, id + 2, top_right, bottom_right);
        self.add_edge(id + 2, id + 3, bottom_right, bottom_left);
        self.add_edge(id + 3, id, bottom_left, top_left);
        self.next_edge_id += 4;
    }

    fn add_edge(
        &mut self,
        mut id: usize,
        mut next: usize,
        mut start: CellCoords,
        mut end: CellCoords,
    ) {
        // Edges must be horizontal or vertical.
        assert!(start.x == end.x || start.y == end.y);

        let mut next_edges = Vec::new();

        for &edge in &self.edges {
            // If the new edge starts at the end of a collinear edge, merge them.
            if edge.end == start && (edge.start.x == end.x || edge.start.y == end.y) {
                start = edge.start;
                id = edge.id;
                continue;
            }
            // If the new edge ends at the start of a collinear edge, merge them.
            if end == edge.start && (start.x == edge.end.x || start.y == edge.end.y) {
                end = edge.end;
                next = edge.next;
                continue;
            }
            next_edges.push(edge);
        }

        if start != end {
            // Include the new edge if it isn't null.
            next_edges.push(Edge {
                id,
                next,
                start,
                end,
            });
        }

        self.edges = next_edges;
    }
}

pub struct LoopsIter {
    edges: HashMap<usize, Edge>,
}

impl LoopsIter {
    pub fn new(edges_vec: Vec<Edge>) -> Self {
        let mut edges = HashMap::new();
        for edge in edges_vec {
            assert!(edges.insert(edge.id, edge).is_none());
        }
        Self { edges }
    }

    pub fn next(&mut self) -> Option<LoopIter<'_>> {
        let first_id = self.edges.iter().min_by_key(|&(&x, _)| x).map(|(&x, _)| x);
        if let Some(first_id) = first_id {
            let loop_iter = LoopIter::new(&mut self.edges, first_id);
            return Some(loop_iter);
        }
        None
    }
}

pub struct LoopIter<'a> {
    next_id: usize,
    edges: &'a mut HashMap<usize, Edge>,
}

impl<'a> LoopIter<'a> {
    pub fn new(edges: &'a mut HashMap<usize, Edge>, first_id: usize) -> Self {
        Self {
            next_id: first_id,
            edges,
        }
    }
}

impl Iterator for LoopIter<'_> {
    type Item = Edge;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(edge) = self.edges.remove(&self.next_id) {
            self.next_id = edge.next;
            return Some(edge);
        }

        None
    }
}

pub struct LoopPairIter<I, T>
where
    I: Iterator<Item = T>,
    T: Clone,
{
    iter: I,
    first: Option<T>,
    prev: Option<T>,
}

impl<I, T> LoopPairIter<I, T>
where
    I: Iterator<Item = T>,
    T: Clone,
{
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            first: None,
            prev: None,
        }
    }
}

impl<I, T> Iterator for LoopPairIter<I, T>
where
    I: Iterator<Item = T>,
    T: Clone,
{
    type Item = (T, T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(next) = self.iter.next() {
                if let Some(prev) = self.prev.take() {
                    self.prev = Some(next.clone());
                    return Some((prev, next));
                } else {
                    self.first = Some(next.clone());
                    self.prev = Some(next);
                    continue;
                }
            } else if let (Some(prev), Some(first)) = (self.prev.take(), self.first.take()) {
                return Some((prev, first));
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perimeter_broken_loop_bug() {
        let mut perimeter = RangeBorder::default();
        perimeter.add_rect(CellCoords { x: 3, y: 2 }, CellCoords { x: 4, y: 3 });
        perimeter.add_rect(CellCoords { x: 0, y: 3 }, CellCoords { x: 1, y: 4 });
        perimeter.add_rect(CellCoords { x: 1, y: 3 }, CellCoords { x: 2, y: 4 });
        perimeter.add_rect(CellCoords { x: 2, y: 3 }, CellCoords { x: 3, y: 4 });
        perimeter.add_rect(CellCoords { x: 3, y: 3 }, CellCoords { x: 4, y: 4 });
        perimeter.add_rect(CellCoords { x: 4, y: 0 }, CellCoords { x: 8, y: 4 });

        let mut loops_iter = LoopsIter::new(perimeter.edges);

        while let Some(loop_iter) = loops_iter.next() {
            println!("***start loop***");
            for edge in loop_iter {
                println!("edge: {:?}", edge);
            }
            println!("***end loop***");
        }
    }
}
