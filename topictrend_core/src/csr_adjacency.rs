/// A reusable Compressed Sparse Row (CSR) Adjacency List.
/// Replaces Vec<Vec<u32>>.
#[derive(Debug)]
pub struct CsrAdjacency {
    // Points to the start index in `targets` for a given ID.
    // Length = num_nodes + 1
    offsets: Vec<usize>,

    // The contiguous list of all edges (children/parents).
    targets: Vec<u32>,
}

impl CsrAdjacency {
    /// Returns the neighbors (children or parents) for a given dense_id.
    /// Returns an empty slice if the ID is out of bounds.
    #[inline(always)]
    pub fn get(&self, id: u32) -> &[u32] {
        // Safety: We use `get` to avoid panics if ID is bad,
        // though in your optimized graph ID should always be valid.
        if let Some(&start) = self.offsets.get(id as usize) {
            // We can safely unwrap the end because offsets len is nodes + 1
            let end = self.offsets[id as usize + 1];
            // Return the slice from the giant targets array
            &self.targets[start..end]
        } else {
            &[]
        }
    }

    /// Optimized Builder: Constructs CSR from unsorted pairs (source -> dest).
    /// This uses a "Bucket Sort" approach (2-pass) to avoid resizing vectors.
    ///
    /// - `num_nodes`: The maximum Dense ID + 1.
    /// - `pairs`: Iterator or Vector of (source, dest).
    pub fn from_pairs(num_nodes: usize, pairs: &[(u32, u32)]) -> Self {
        // 1. Pass 1: Count degrees (Frequency Histogram)
        // We need to know how many children each node has to calculate offsets.
        let mut counts = vec![0u32; num_nodes];
        for &(src, _) in pairs {
            if (src as usize) < num_nodes {
                counts[src as usize] += 1;
            }
        }

        // 2. Build Offsets (Cumulative Sum)
        let mut offsets = Vec::with_capacity(num_nodes + 1);
        let mut current_offset = 0;
        offsets.push(0);

        for count in counts {
            current_offset += count as usize;
            offsets.push(current_offset);
        }

        // 3. Pass 2: Populate Targets
        // We allocate the exact size needed.
        let total_edges = offsets[num_nodes];
        let mut targets = vec![0u32; total_edges];

        // We need a running cursor to know where to write the next child for each parent
        // We essentially copy `offsets` to use as write pointers.
        let mut write_cursors = offsets.clone();

        for &(src, dst) in pairs {
            if (src as usize) < num_nodes {
                let pos = write_cursors[src as usize];
                targets[pos] = dst;
                write_cursors[src as usize] += 1;
            }
        }

        CsrAdjacency { offsets, targets }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_csr() {
        let csr = CsrAdjacency::from_pairs(3, &[]);
        assert_eq!(csr.get(0), &[] as &[u32]);
        assert_eq!(csr.get(1), &[] as &[u32]);
        assert_eq!(csr.get(2), &[] as &[u32]);
    }

    #[test]
    fn test_single_edge() {
        let pairs = vec![(0, 5)];
        let csr = CsrAdjacency::from_pairs(2, &pairs);
        assert_eq!(csr.get(0), &[5]);
        assert_eq!(csr.get(1), &[] as &[u32]);
    }

    #[test]
    fn test_multiple_edges_same_source() {
        let pairs = vec![(0, 1), (0, 2), (0, 3)];
        let csr = CsrAdjacency::from_pairs(2, &pairs);
        let neighbors = csr.get(0);
        assert_eq!(neighbors.len(), 3);
        assert!(neighbors.contains(&1));
        assert!(neighbors.contains(&2));
        assert!(neighbors.contains(&3));
        assert_eq!(csr.get(1), &[] as &[u32]);
    }

    #[test]
    fn test_multiple_sources() {
        let pairs = vec![(0, 10), (1, 20), (2, 30), (1, 21)];
        let csr = CsrAdjacency::from_pairs(3, &pairs);
        assert_eq!(csr.get(0), &[10]);
        let node1_neighbors = csr.get(1);
        assert_eq!(node1_neighbors.len(), 2);
        assert!(node1_neighbors.contains(&20));
        assert!(node1_neighbors.contains(&21));
        assert_eq!(csr.get(2), &[30]);
    }

    #[test]
    fn test_out_of_bounds_source() {
        let pairs = vec![(0, 1), (5, 2)]; // 5 is out of bounds for num_nodes=3
        let csr = CsrAdjacency::from_pairs(3, &pairs);
        assert_eq!(csr.get(0), &[1]);
        assert_eq!(csr.get(1), &[] as &[u32]);
        assert_eq!(csr.get(2), &[] as &[u32]);
    }

    #[test]
    fn test_get_out_of_bounds_id() {
        let pairs = vec![(0, 1), (1, 2)];
        let csr = CsrAdjacency::from_pairs(2, &pairs);
        assert_eq!(csr.get(0), &[1]);
        assert_eq!(csr.get(1), &[2]);
        assert_eq!(csr.get(10), &[] as &[u32]); // Out of bounds
    }

    #[test]
    fn test_duplicate_pairs() {
        let pairs = vec![(0, 1), (0, 1), (1, 2)];
        let csr = CsrAdjacency::from_pairs(2, &pairs);
        assert_eq!(csr.get(0), &[1, 1]); // Duplicates are preserved
        assert_eq!(csr.get(1), &[2]);
    }

    #[test]
    fn test_large_node_ids() {
        let pairs = vec![(0, 100), (1, 200)];
        let csr = CsrAdjacency::from_pairs(2, &pairs);
        assert_eq!(csr.get(0), &[100]);
        assert_eq!(csr.get(1), &[200]);
    }
}
