use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

struct Inner<T> {
    token: T,
    created: Instant,
}

impl<T> Debug for Inner<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.token, f)
    }
}

#[derive(Debug)]
pub struct OnceToken<T, G = fn() -> T> {
    inner: Mutex<Option<Inner<T>>>,
    generator: G,
    ttl: Duration,
}

impl<T, G> OnceToken<T, G> {
    pub fn new(ttl: Duration, generator: G) -> Self {
        Self {
            inner: Mutex::new(None),
            ttl,
            generator,
        }
    }

    pub fn set(&self, token: T) {
        self.inner().replace(Inner {
            created: Instant::now(),
            token,
        });
    }

    /// Compare given token with saved token, if they are equal, remove saved token
    pub fn eq_once<U>(&self, token: &U) -> bool
    where
        T: Borrow<U>,
        U: PartialEq + ?Sized,
    {
        let inner = &mut *self.inner();
        let token = token.borrow();
        let expired = inner.take_if(|v| v.created.elapsed() > self.ttl).is_some();
        if expired {
            false // expired token is unauthorized
        } else {
            inner.take_if(|v| v.token.borrow() == token).is_some()
        }
    }

    #[allow(dead_code)]
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    fn inner(&self) -> MutexGuard<'_, Option<Inner<T>>> {
        // Poisoned state is not a problem for us.
        self.inner.lock().unwrap_or_else(|x| x.into_inner())
    }
}

impl<T, G> OnceToken<T, G>
where
    T: Clone,
    G: Fn() -> T,
{
    #[must_use]
    pub fn generate(&self) -> T {
        let token = (self.generator)();
        self.set(token.clone());
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Add;
    use uuid::Uuid;

    const TTL: Duration = Duration::from_secs(10);

    #[test]
    fn test_once_token() {
        let ott: OnceToken<Uuid> = OnceToken::new(TTL, Uuid::new_v4);

        // No token
        assert!(!ott.eq_once(&Uuid::new_v4()));

        let token = ott.generate();
        // first call should return true
        assert!(ott.inner().is_some());
        assert!(ott.eq_once(&token), "unexpired and authorized token");
        assert!(ott.inner().is_none(), "token is removed after used");
        // second call of unexpired token should fail
        assert!(!ott.eq_once(&token), "used token is unauthorized");
    }

    #[test]
    fn test_once_token_of_string() {
        let ott: OnceToken<String> = OnceToken::new(TTL, String::new);

        // No token
        assert!(!ott.eq_once(&String::new()));

        let token = Uuid::new_v4().to_string();
        ott.set(token.clone());
        assert!(ott.eq_once(token.as_str()));
    }

    #[test]
    fn test_expired_once_token() {
        let ott: OnceToken<Uuid> = OnceToken::new(TTL, Uuid::new_v4);

        let token = create_expired_token(&ott);
        assert!(ott.inner().is_some());
        assert!(!ott.eq_once(&token), "expired token");
        assert!(ott.inner().is_none());

        let _token = create_expired_token(&ott);
        assert!(!ott.eq_once(&Uuid::new_v4()), "unmatched and expired token");
        assert!(ott.inner().is_none());
    }

    fn create_expired_token(ott: &OnceToken<Uuid>) -> Uuid {
        let token = ott.generate();
        let mut guard = ott.inner();
        let inner = guard.as_mut().unwrap();
        // change created instant to some moment before random was called.
        let long_before_created = inner.created.checked_sub(TTL.add(TTL)).unwrap();
        inner.created = long_before_created;
        token
    }
}
