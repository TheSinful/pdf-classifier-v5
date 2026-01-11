use std::ops::Deref;

/// Ensures that inner is always Some
/// Used in static arrays where members must be initialized as None but are guaranteed to be Some later on
#[derive(Clone, Copy)]
pub struct AlwaysSome<T> {
    pub inner: Option<T>,
}

impl<T> From<Option<T>> for AlwaysSome<T> {
    fn from(value: Option<T>) -> Self {
        Self { inner: value }
    }
}   

impl<T> From<T> for AlwaysSome<T> {
    fn from(value: T) -> Self {
        Self { inner: Some(value) }
    }
}

impl<T> Deref for AlwaysSome<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
