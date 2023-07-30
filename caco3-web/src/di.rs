//! Inversion of control.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::panic::Location;
use std::sync::{Arc, OnceLock};

use http::Extensions;
use thiserror::Error;
use tracing::warn;

/// Wrapper type for managing dependency.
#[derive(Debug)]
pub struct Dep<T: 'static + ?Sized>(DepInner<T>);

// implementation detail of Dep
#[derive(Debug, strum::IntoStaticStr)]
enum DepInner<T: 'static + ?Sized> {
    Arc(Arc<T>),
    LazyArc(OnceLock<Arc<T>>),
}

impl<T: Sized> Dep<T> {
    /// Create a new dependency.
    pub fn new(val: T) -> Self {
        Self::new_arc(Arc::new(val))
    }
}

impl<T: ?Sized> Dep<T> {
    pub fn new_arc(arc: Arc<T>) -> Self {
        Self(DepInner::Arc(arc))
    }

    pub fn lazy() -> Self {
        Self(DepInner::LazyArc(OnceLock::new()))
    }

    pub fn try_as_ref(this: &Self) -> Result<&T, AsRefError<T>> {
        match &this.0 {
            DepInner::Arc(arc) => Ok(arc),
            DepInner::LazyArc(cell) => cell.get().map(Arc::as_ref).ok_or_else(AsRefError::new),
        }
    }

    #[track_caller]
    pub fn bind(src: &Self, dst: &Self) {
        if let Err(err) = Self::try_bind(src, dst) {
            handle_bind_error::<T>(err)
        }
    }

    pub fn try_bind(src: &Self, dst: &Self) -> Result<(), BindError> {
        use BindError::*;
        match (&src.0, &dst.0) {
            (DepInner::LazyArc(src_cell), DepInner::LazyArc(dst_cell)) => {
                let src_arc = src_cell.get().ok_or(UninitializedSourceCell)?.clone();
                dst_cell
                    .set(src_arc)
                    .map_err(|_| InitializedDestinationCell)?;
            }
            (DepInner::Arc(src_arc), DepInner::LazyArc(dst_cell)) => {
                dst_cell
                    .set(src_arc.clone())
                    .map_err(|_| InitializedDestinationCell)?;
            }
            _ => {
                return Err(IncompatibleVariant {
                    src: From::from(&src.0),
                    dst: From::from(&dst.0),
                })
            }
        }
        Ok(())
    }

    /// Returns `true` if `this` is initialized.
    pub fn is_initialized(this: &Self) -> bool {
        match &this.0 {
            DepInner::Arc(..) => true,
            DepInner::LazyArc(cell) => cell.get().is_some(),
        }
    }

    pub fn assert_initialized(this: &Self) {
        assert!(Self::is_initialized(this), "cell is uninitialized")
    }

    pub fn as_arc(this: &Self) -> Option<&Arc<T>> {
        let arc = match &this.0 {
            DepInner::Arc(arc) => arc,
            DepInner::LazyArc(cell) => cell.get()?,
        };
        Some(arc)
    }

    pub fn try_with<F, R>(&self, f: F) -> Result<R, ()>
    where
        F: FnOnce(&T) -> R,
    {
        Dep::try_as_ref(self).map(f).map_err(|_| ())
    }
}

#[track_caller]
fn handle_bind_error<T: ?Sized>(err: BindError) {
    match err {
        BindError::InitializedDestinationCell => {
            let caller = Location::caller();
            warn!(
                "Bind already initialized instance of {} at {file}:{line}",
                std::any::type_name::<T>(),
                file = caller.file(),
                line = caller.line(),
            )
        }
        err => {
            panic!("BindError: {}", err);
        }
    }
}

impl<T: ?Sized> Clone for DepInner<T> {
    fn clone(&self) -> Self {
        match self {
            DepInner::Arc(arc) => DepInner::Arc(arc.clone()),
            DepInner::LazyArc(cell) => DepInner::LazyArc(cell.clone()),
        }
    }
}

impl<T: ?Sized> Clone for Dep<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> From<T> for Dep<T> {
    fn from(val: T) -> Self {
        Self(DepInner::Arc(Arc::new(val)))
    }
}

impl<T: ?Sized> From<Arc<T>> for Dep<T> {
    fn from(val: Arc<T>) -> Self {
        Self(DepInner::Arc(val))
    }
}

impl<T: ?Sized> Deref for Dep<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Dep::try_as_ref(self).expect("initialized dependency")
    }
}

#[derive(Error)]
pub enum BindError {
    #[error("destination cell is already initialized")]
    InitializedDestinationCell,
    #[error("source cell is uninitialized")]
    UninitializedSourceCell,
    #[error("incompatible variant, src variant: {src}, dst variant: {dst}")]
    IncompatibleVariant {
        src: &'static str,
        dst: &'static str,
    },
}

impl fmt::Debug for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Error)]
#[error("Dependency of type {} is uninitialized", std::any::type_name::<T>())]
pub struct AsRefError<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> AsRefError<T> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: ?Sized> fmt::Debug for AsRefError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Support late dependency binding at runtime.
pub trait BindDep {
    fn bind_dep(&self, map: &TypeMap);
}

/// A type map of dependencies.
#[derive(Default)]
pub struct TypeMap(Extensions);

impl TypeMap {
    pub fn new() -> Self {
        Default::default()
    }

    /// Get a reference to a type previously inserted on this Map.
    ///
    /// panic if an instance of type doesn't exist.
    pub fn get_instance<T: Send + Sync + 'static>(&self) -> &T {
        self.0.get().unwrap_or_else(|| {
            panic!(
                r##"Not found type: "{}" in TypeMap"##,
                std::any::type_name::<T>()
            );
        })
    }

    #[track_caller]
    pub fn bind_instance<T: Send + Sync + 'static>(&self, target: &Dep<T>) {
        let source: &Dep<T> = self.get_instance();
        if let Err(err) = Dep::try_bind(source, target) {
            handle_bind_error::<T>(err);
        }
    }

    /// Get a reference to inner extensions.
    pub fn extensions(&self) -> &Extensions {
        &self.0
    }
}

impl From<Extensions> for TypeMap {
    fn from(ext: Extensions) -> Self {
        Self(ext)
    }
}

impl Deref for TypeMap {
    type Target = Extensions;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TypeMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_assert_initialized_lazy_arc() {
        let a = Dep::<()>::lazy();
        Dep::assert_initialized(&a);
    }

    #[test]
    fn test_cyclic_dependency() {
        struct Foo {
            bar: Dep<Bar>,
        }

        impl BindDep for Foo {
            fn bind_dep(&self, map: &TypeMap) {
                map.bind_instance(&self.bar);
            }
        }

        struct Bar {
            foo: Dep<Foo>,
        }

        let foo = Dep::new(Foo { bar: Dep::lazy() });
        let bar = Dep::new(Bar { foo: foo.clone() });

        let mut map = TypeMap::new();
        map.insert(bar.clone());
        foo.bind_dep(&map);
        Dep::assert_initialized(&foo.bar);
        Dep::assert_initialized(&bar.foo);
    }
}
