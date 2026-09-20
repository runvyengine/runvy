use std::sync::Arc;

#[derive(Debug)]
pub struct Handle<T> {
    pub inner: Arc<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> From<Arc<T>> for Handle<T> {
    fn from(arc: Arc<T>) -> Self {
        Self { inner: arc }
    }
}

impl<T> From<Handle<T>> for Arc<T> {
    fn from(handle: Handle<T>) -> Self {
        handle.inner
    }
}
