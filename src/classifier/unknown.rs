/// Wrapper struct to state T will exist at some point, just doesn't exist yet
#[derive(Clone, Copy)]
pub struct Unknown<T> {
    inner: Option<T>,
}

impl<T> Unknown<T> {
    pub fn pending() -> Self {
        Self { inner: None }
    }

    pub fn define(&mut self, inner: T) -> () {
        self.inner = Some(inner);
    }

    pub fn is_pending(&self) -> bool {
        self.inner.is_none()
    }

    pub fn unwrap(self) -> T {
        self.inner.expect("Attempted to unwrap on a pending value!")
    }
}
