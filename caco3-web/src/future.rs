use std::future::Future;
use std::thread::panicking;

impl<Fut> OnUncompletedDrop for Fut where Fut: Future + Send {}

pub trait OnUncompletedDrop: Future + Send + Sized {
    /// Call given closure `f` when uncompleted future is dropped.
    ///
    /// if `panic` is true, `f` is also called on panic.
    fn on_uncompleted_drop<F>(self, panic: bool, f: F) -> impl Future<Output = Self::Output> + Send
    where
        F: FnOnce() + Send,
    {
        let drop_guard = UncompletedDropGuard { f: Some(f), panic };
        async move {
            let output = self.await;
            core::mem::forget(drop_guard);
            output
        }
    }
}

struct UncompletedDropGuard<F>
where
    F: FnOnce() + Send,
{
    f: Option<F>,
    panic: bool,
}

impl<F> Drop for UncompletedDropGuard<F>
where
    F: FnOnce() + Send,
{
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            if self.panic || !panicking() {
                f()
            }
        }
    }
}

impl<Fut> OnUncompletedDropLocal for Fut where Fut: Future {}

pub trait OnUncompletedDropLocal: Future + Sized {
    /// Call given closure `f` when uncompleted future is dropped.
    ///
    /// if `panic` is true, `f` is also called on panic.
    fn on_uncompleted_drop_local<F: FnOnce()>(
        self,
        panic: bool,
        f: F,
    ) -> impl Future<Output = Self::Output> {
        let drop_guard = UncompletedDropGuardLocal { f: Some(f), panic };
        async move {
            let output = self.await;
            core::mem::forget(drop_guard);
            output
        }
    }
}

struct UncompletedDropGuardLocal<F>
where
    F: FnOnce(),
{
    f: Option<F>,
    panic: bool,
}

impl<F> Drop for UncompletedDropGuardLocal<F>
where
    F: FnOnce(),
{
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            if self.panic || !panicking() {
                f()
            }
        }
    }
}
