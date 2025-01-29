
#[derive(Default)]
struct RangeBorder {
    next_edge_id: usize,
    edges: Vec<Edge>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct Edge {
    id: usize,
    next: usize,
    start: (u64, u64),
    end: (u64, u64),
}

impl RangeBorder {
    fn add_rect(&mut self, top: u64, left: u64, bottom: u64, right: u64) {
        let id = self.next_edge_id;
        self.add_edge(id, id + 1, (left, top), (right, top));
        self.add_edge(id + 1, id + 2, (right, top), (right, bottom));
        self.add_edge(id + 2, id + 3, (right, bottom), (left, bottom));
        self.add_edge(id + 3, id, (left, bottom), (left, top));
        self.next_edge_id += 4;
    }

    fn add_edge(
        &mut self,
        mut id: usize,
        mut next: usize,
        mut start: (u64, u64),
        mut end: (u64, u64),
    ) {
        // Edges must be horizontal or vertical.
        assert!(start.0 == end.0 || start.1 == end.1);

        let mut next_edges = Vec::new();

        for &edge in &self.edges {
            // If the new edge starts at the end of a collinear edge, merge them.
            if edge.end == start && (edge.start.0 == end.0 || edge.start.1 == end.1) {
                start = edge.start;
                id = edge.id;
                continue;
            }
            // If the new edge ends at the start of a collinear edge, merge them.
            if end == edge.start && (start.0 == edge.end.0 || start.1 == edge.end.1) {
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

struct LoopsIter {
    edges: HashMap<usize, Edge>,
}

impl LoopsIter {
    fn new(edges_vec: Vec<Edge>) -> Self {
        let mut edges = HashMap::new();
        for edge in edges_vec {
            assert!(edges.insert(edge.id, edge).is_none());
        }
        Self { edges }
    }

    fn next(&mut self) -> Option<LoopIter<'_>> {
        let first_id = self.edges.iter().min_by_key(|&(&x, _)| x).map(|(&x, _)| x);
        if let Some(first_id) = first_id {
            let loop_iter = LoopIter::new(&mut self.edges, first_id);
            return Some(loop_iter);
        }
        None
    }
}

struct LoopIter<'a> {
    next_id: usize,
    edges: &'a mut HashMap<usize, Edge>,
}

impl<'a> LoopIter<'a> {
    fn new(edges: &'a mut HashMap<usize, Edge>, first_id: usize) -> Self {
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

struct LoopPairIter<I, T>
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
    fn new(iter: I) -> Self {
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
        perimeter.add_rect(2, 3, 3, 4);
        perimeter.add_rect(3, 0, 4, 1);
        perimeter.add_rect(3, 1, 4, 2);
        perimeter.add_rect(3, 2, 4, 3);
        perimeter.add_rect(3, 3, 4, 4);
        perimeter.add_rect(0, 4, 4, 8);

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
