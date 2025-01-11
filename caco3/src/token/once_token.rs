use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

struct TokenData<T> {
    token: T,
    created: Instant,
}

impl<T> Debug for TokenData<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.token, f)
    }
}

#[derive(Debug)]
pub struct OnceToken<T, G = fn() -> T> {
    data: Mutex<Option<TokenData<T>>>,
    generator: G,
    ttl: Duration,
}

impl<T, G> OnceToken<T, G> {
    pub fn new(ttl: Duration, generator: G) -> Self {
        Self {
            data: Mutex::new(None),
            generator,
            ttl,
        }
    }

    pub fn set(&self, token: T) {
        self.data().replace(TokenData {
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
        let data = &mut *self.data();
        let expired = data.take_if(|v| v.created.elapsed() > self.ttl).is_some();
        if expired {
            false // expired token is unauthorized
        } else {
            data.take_if(|v| v.token.borrow() == token).is_some()
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    fn data(&self) -> MutexGuard<'_, Option<TokenData<T>>> {
        // Poisoned state is not a problem for us.
        self.data.lock().unwrap_or_else(|x| x.into_inner())
    }
}

impl<T, G> OnceToken<T, G>
where
    T: Clone,
    G: Fn() -> T,
{
    #[must_use]
    pub fn generate(&self) -> T {
        let new_token = (self.generator)();
        self.set(new_token.clone());
        new_token
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
        let ot: OnceToken<Uuid> = OnceToken::new(TTL, Uuid::new_v4);

        // No token
        assert!(!ot.eq_once(&Uuid::new_v4()));

        let token = ot.generate();
        // first call should return true
        assert!(ot.data().is_some());
        assert!(ot.eq_once(&token), "unexpired and authorized token");
        assert!(ot.data().is_none(), "token is removed after used");
        // second call of unexpired token should fail
        assert!(!ot.eq_once(&token), "used token is unauthorized");
    }

    #[test]
    fn test_once_token_of_string() {
        let ot: OnceToken<String> = OnceToken::new(TTL, String::new);

        // No token
        assert!(!ot.eq_once(&String::new()));

        let token = Uuid::new_v4().to_string();
        ot.set(token.clone());
        assert!(ot.eq_once(token.as_str()));
    }

    #[test]
    fn test_expired_once_token() {
        let ot: OnceToken<Uuid> = OnceToken::new(TTL, Uuid::new_v4);

        let token = create_expired_token(&ot);
        assert!(ot.data().is_some());
        assert!(!ot.eq_once(&token), "expired token");
        assert!(ot.data().is_none());

        let _token = create_expired_token(&ot);
        assert!(!ot.eq_once(&Uuid::new_v4()), "unmatched and expired token");
        assert!(ot.data().is_none());
    }

    fn create_expired_token(ot: &OnceToken<Uuid>) -> Uuid {
        let token = ot.generate();
        let mut data = ot.data();
        let data = data.as_mut().unwrap();
        // change created instant to some moment before random was called.
        let long_before_created = data.created.checked_sub(TTL.add(TTL)).unwrap();
        data.created = long_before_created;
        token
    }
}
