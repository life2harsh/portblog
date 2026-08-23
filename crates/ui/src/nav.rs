#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDir {
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub struct KeyNav {
    pub index: usize,
    pub len: usize,
}

impl KeyNav {
    pub fn new(len: usize) -> Self {
        Self { index: 0, len }
    }

    pub fn advance(&self, dir: NavDir) -> Self {
        let index = match dir {
            NavDir::Up => self.index.saturating_sub(1),
            NavDir::Down => (self.index + 1).min(self.len.saturating_sub(1)),
        };
        Self {
            index,
            len: self.len,
        }
    }

    pub fn with_len(&self, len: usize) -> Self {
        Self {
            index: self.index.min(len.saturating_sub(1)),
            len,
        }
    }
}
