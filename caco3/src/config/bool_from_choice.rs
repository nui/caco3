use std::fmt;

use serde::de::{Expected, Unexpected};
use serde::{de, Deserializer};

use crate::config::{is_falsy, is_truthy};

struct TruthyOrFalsy;

impl Expected for TruthyOrFalsy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut values = super::TRUTHY_VALUES
            .iter()
            .chain(super::FALSY_VALUES.iter())
            .copied();
        let first = values.next().unwrap();
        write!(formatter, r##"Any of ["{}""##, first)?;
        for v in values {
            write!(formatter, r##", "{v}""##)?;
        }
        write!(formatter, "] (case-insensitive)")
    }
}

pub fn bool_from_choice<'de, D: Deserializer<'de>>(de: D) -> Result<bool, D::Error> {
    struct Visitor;

    impl de::Visitor<'_> for Visitor {
        type Value = bool;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a boolean")
        }

        fn visit_bool<E: de::Error>(self, b: bool) -> Result<bool, E> {
            Ok(b)
        }

        fn visit_i64<E: de::Error>(self, n: i64) -> Result<bool, E> {
            match n {
                0 | 1 => Ok(n != 0),
                n => Err(E::invalid_value(Unexpected::Signed(n), &"0 or 1")),
            }
        }

        fn visit_u64<E: de::Error>(self, n: u64) -> Result<bool, E> {
            match n {
                0 | 1 => Ok(n != 0),
                n => Err(E::invalid_value(Unexpected::Unsigned(n), &"0 or 1")),
            }
        }

        fn visit_str<E: de::Error>(self, val: &str) -> Result<bool, E> {
            match val {
                v if is_truthy(v) => Ok(true),
                v if is_falsy(v) => Ok(false),
                s => Err(E::invalid_value(Unexpected::Str(s), &TruthyOrFalsy)),
            }
        }
    }

    de.deserialize_any(Visitor)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_test::{assert_de_tokens, Token};

    use super::*;

    #[derive(Deserialize, Debug, Eq, PartialEq)]
    #[serde(transparent)]
    struct Bool(#[serde(deserialize_with = "bool_from_choice")] bool);

    #[test]
    fn test_deserialize() {
        let falsy = Bool(false);
        assert_de_tokens(&falsy, &[Token::Str("0")]);
        assert_de_tokens(&falsy, &[Token::Str("false")]);
        assert_de_tokens(&falsy, &[Token::Str("False")]);
        assert_de_tokens(&falsy, &[Token::Str("n")]);
        assert_de_tokens(&falsy, &[Token::Str("N")]);
        assert_de_tokens(&falsy, &[Token::Str("no")]);
        assert_de_tokens(&falsy, &[Token::Str("off")]);
        assert_de_tokens(&falsy, &[Token::U64(0)]);
        assert_de_tokens(&falsy, &[Token::Bool(false)]);

        let truthy = Bool(true);
        assert_de_tokens(&truthy, &[Token::Str("1")]);
        assert_de_tokens(&truthy, &[Token::Str("true")]);
        assert_de_tokens(&truthy, &[Token::Str("True")]);
        assert_de_tokens(&truthy, &[Token::Str("y")]);
        assert_de_tokens(&truthy, &[Token::Str("Y")]);
        assert_de_tokens(&truthy, &[Token::Str("yes")]);
        assert_de_tokens(&truthy, &[Token::Str("on")]);
        assert_de_tokens(&truthy, &[Token::U64(1)]);
        assert_de_tokens(&truthy, &[Token::Bool(true)]);
    }
}
