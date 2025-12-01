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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_map() {
        let map = DirectMap::new(10);
        assert_eq!(map.get(0), None);
        assert_eq!(map.get(5), None);
        assert_eq!(map.get(10), None);
    }

    #[test]
    fn test_insert_and_get() {
        let mut map = DirectMap::new(10);
        map.insert(5, 100);
        assert_eq!(map.get(5), Some(100));
        assert_eq!(map.get(4), None);
        assert_eq!(map.get(6), None);
    }

    #[test]
    fn test_insert_overwrites_existing_value() {
        let mut map = DirectMap::new(10);
        map.insert(3, 50);
        map.insert(3, 75);
        assert_eq!(map.get(3), Some(75));
    }

    #[test]
    fn test_resize_on_large_key() {
        let mut map = DirectMap::new(5);
        map.insert(10, 200);
        assert_eq!(map.get(10), Some(200));
        assert_eq!(map.get(5), None);
    }

    #[test]
    fn test_multiple_insertions() {
        let mut map = DirectMap::new(20);
        map.insert(0, 10);
        map.insert(15, 150);
        map.insert(7, 70);
        
        assert_eq!(map.get(0), Some(10));
        assert_eq!(map.get(15), Some(150));
        assert_eq!(map.get(7), Some(70));
        assert_eq!(map.get(1), None);
        assert_eq!(map.get(8), None);
    }

    #[test]
    fn test_get_out_of_bounds() {
        let map = DirectMap::new(5);
        assert_eq!(map.get(100), None);
    }
}
