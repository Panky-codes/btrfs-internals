use core::fmt;
use std::vec::Vec;

#[derive(Default, Clone, Copy)]
pub struct ChunkTreeKey {
    pub start: u64,
    pub size: u64,
}
pub struct ChunkTree {
    vec: Vec<(ChunkTreeKey, u64)>,
}

impl ChunkTree {
    pub fn new() -> ChunkTree {
        ChunkTree { vec: vec![] }
    }

    fn check_for_overlap(&self, key: &ChunkTreeKey) -> bool {
        let key_range_end = key.start + key.size;
        for (k, _) in &self.vec {
            if key.start >= k.start && key.start < (k.start + k.size)
                || key_range_end >= k.start && key_range_end < (k.start + k.size)
            {
                return true;
            }
        }
        return false;
    }

    pub fn find_logical(&self, logical: u64) -> Option<(ChunkTreeKey, u64)> {
        for (key, off) in &self.vec {
            if logical >= key.start && logical < (key.start + key.size) {
                return Some((*key, *off));
            }
        }
        None
    }

    pub fn offset(&self, logical: u64) -> Option<u64> {
        if let Some((k, v)) = self.find_logical(logical) {
            return Some(v + (logical - k.start));
        }
        None
    }
    pub fn insert(&mut self, key: ChunkTreeKey, offset: u64) -> Result<i32, i32> {
        if self.check_for_overlap(&key) {
            println!("Overlapping chunks");
            return Err(1);
        }

        self.vec.push((key, offset));
        Ok(0)
    }
}

impl fmt::Display for ChunkTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        for (chunk, off) in &self.vec {
            write!(
                f,
                "Logical start {}, Logical size {}, Physical off: {}",
                chunk.start, chunk.size, off
            )?;
        }

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::ChunkTree;

    #[test]
    fn check_overlap() {
        let mut chunk: ChunkTree = ChunkTree::new();

        let _insert1 = chunk.insert(
            super::ChunkTreeKey {
                start: 200,
                size: 100,
            },
            5,
        );
        let insert2 = chunk.insert(
            super::ChunkTreeKey {
                start: 250,
                size: 100,
            },
            6,
        );
        assert_eq!(insert2.err(), Some(1));
    }

    #[test]
    fn check_overlap1() {
        let mut chunk: ChunkTree = ChunkTree::new();

        let _insert1 = chunk.insert(
            super::ChunkTreeKey {
                start: 200,
                size: 100,
            },
            5,
        );
        let insert2 = chunk.insert(
            super::ChunkTreeKey {
                start: 150,
                size: 100,
            },
            6,
        );
        assert_eq!(insert2.err(), Some(1));
    }

    #[test]
    fn check_overlap2() {
        let mut chunk: ChunkTree = ChunkTree::new();

        let _insert1 = chunk.insert(
            super::ChunkTreeKey {
                start: 200,
                size: 100,
            },
            5,
        );
        let insert2 = chunk.insert(
            super::ChunkTreeKey {
                start: 350,
                size: 100,
            },
            6,
        );
        assert_eq!(insert2.err(), None);
    }
}
