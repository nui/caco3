use figment::providers::Serialized;
use figment::Figment;
use serde_json::Value;
use thiserror::Error;

mod private {
    pub trait Sealed {}

    impl Sealed for figment::Figment {}
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RemoveExistingKeyError<'a> {
    #[error("key {0} not found")]
    NotFound(&'a str),
}

/// Extension trait for `figment::Figment`.
pub trait FigmentExt: Sized + private::Sealed {
    /// Remove existing keys.
    ///
    /// This method error if key doesn't exist.
    fn remove_existing_keys<'a, T: AsRef<str>>(
        &self,
        keys: &'a [T],
    ) -> Result<Self, RemoveExistingKeyError<'a>>;

    /// Check for key existent.
    ///
    /// blank key return `false`.
    fn has_key(&self, key: &str) -> bool;
}

impl FigmentExt for Figment {
    fn remove_existing_keys<'a, T: AsRef<str>>(
        &self,
        keys: &'a [T],
    ) -> Result<Self, RemoveExistingKeyError<'a>> {
        let mut value = self.extract::<Value>().expect("json serializable value");
        let mut pointer = String::new();
        let mut parts = vec![];
        for key in keys {
            let key = key.as_ref();
            if !self.has_key(key) {
                return Err(RemoveExistingKeyError::NotFound(key));
            }
            pointer.clear();
            parts.clear();
            parts.extend(key.split('.'));
            // note: .expect("object") should never fail because we already check key existent
            match parts.as_slice() {
                [] => {
                    // we already check key existent
                    unreachable!("empty parts");
                }
                [field] => {
                    value.as_object_mut().expect("object").remove(*field);
                }
                [components @ .., field] => {
                    for c in components {
                        pointer.push('/');
                        pointer.push_str(c);
                    }
                    value
                        .pointer_mut(&pointer)
                        .and_then(Value::as_object_mut)
                        .expect("object")
                        .remove(*field);
                }
            }
        }
        Ok(Figment::from(Serialized::defaults(value)))
    }

    fn has_key(&self, key: &str) -> bool {
        self.find_metadata(key).is_some() && !key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_figment() -> Figment {
        Figment::from(Serialized::defaults(serde_json::json!({
            "foo": {
                "bar": {
                    "baz": {
                        "name": "Baz",
                    }
                },
                "s": "Foo string",
            },
            "vec": ["foo", "bar", "baz"],
        })))
    }

    #[test]
    fn remove_existing_keys() {
        let figment = get_test_figment();

        #[rustfmt::skip]
        let keys = [
            "foo.bar.baz.name",
            "foo.s",
        ];
        for key in keys {
            assert!(figment.has_key(key));
        }
        let actual = figment.remove_existing_keys(&keys);
        let f = actual.expect("keys removed");
        for key in keys {
            assert!(!f.has_key(key));
        }
    }

    #[test]
    fn remove_missing_key() {
        let figment = get_test_figment();
        let key = "foo.not_exist";
        assert!(!figment.has_key(key));
        let keys = [key];
        let actual = figment.remove_existing_keys(&keys);
        let err = actual.expect_err("key doesn't exist");
        assert!(matches!(err, RemoveExistingKeyError::NotFound(_)));
    }

    #[test]
    fn has_key() {
        let figment = get_test_figment();
        assert!(!figment.has_key(""));
    }
}
