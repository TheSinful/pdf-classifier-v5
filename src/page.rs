
#[derive(Clone, Copy)]
pub struct Page {
    pub num: u32,
}

impl From<u32> for Page {
    fn from(value: u32) -> Self {
        Self { num: value }
    }
}

impl Into<u32> for Page {
    fn into(self) -> u32 {
        self.num
    }
}