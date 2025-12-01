#[derive(Debug)]
pub struct DirectMap {
    mapping: Vec<u32>,
}

impl DirectMap {
    pub fn new(max_size: usize) -> Self {
        // Initialize with u32::MAX as a sentinel for "Not Found"
        Self {
            mapping: vec![u32::MAX; max_size + 1],
        }
    }

    pub fn insert(&mut self, key: u32, value: u32) {
        if (key as usize) >= self.mapping.len() {
            // Resize if we see an unexpectedly large ID
            self.mapping.resize(key as usize + 1, u32::MAX);
        }
        self.mapping[key as usize] = value;
    }

    #[inline(always)]
    pub fn get(&self, key: u32) -> Option<u32> {
        match self.mapping.get(key as usize) {
            Some(&val) if val != u32::MAX => Some(val),
            _ => None,
        }
    }
}
