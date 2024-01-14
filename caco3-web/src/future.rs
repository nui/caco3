use std::future::Future;
use std::thread::panicking;

impl<T> OnUncompletedDrop for T where T: Future {}

pub trait OnUncompletedDrop: Future + Sized {
    /// Call given closure `f` when uncompleted future is dropped.
    ///
    /// if `panic` is true, `f` is also called on panic.
    fn on_uncompleted_drop<F: FnOnce() -> ()>(
        self,
        panic: bool,
        f: F,
    ) -> impl Future<Output = Self::Output> {
        let guard = UncompletedDropGuard { f: Some(f), panic };
        async move {
            let output = self.await;
            core::mem::forget(guard);
            output
        }
    }
}

struct UncompletedDropGuard<F: FnOnce() -> ()> {
    f: Option<F>,
    panic: bool,
}

impl<F: FnOnce() -> ()> Drop for UncompletedDropGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            if self.panic || !panicking() {
                f()
            }
        }
    }
}
