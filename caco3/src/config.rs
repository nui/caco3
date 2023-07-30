pub use bool_from_choice::bool_from_choice;
pub use meta::MetaConfig;

mod bool_from_choice;
mod meta;

const FALSY_VALUES: &[&str] = &["0", "false", "n", "no", "off"];
const TRUTHY_VALUES: &[&str] = &["1", "true", "y", "yes", "on"];

pub fn is_falsy<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();
    FALSY_VALUES.iter().any(|s| value.eq_ignore_ascii_case(s))
}

pub fn is_truthy<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();
    TRUTHY_VALUES.iter().any(|s| value.eq_ignore_ascii_case(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_yes() {
        assert!(is_truthy("1"));
        assert!(is_truthy("true"));
        assert!(is_truthy("y"));
        assert!(is_truthy("Y"));
        assert!(is_truthy("yes"));
        assert!(is_truthy("on"));

        assert!(!is_truthy("n"));
    }

    #[test]
    fn test_is_no() {
        assert!(is_falsy("0"));
        assert!(is_falsy("false"));
        assert!(is_falsy("n"));
        assert!(is_falsy("N"));
        assert!(is_falsy("no"));
        assert!(is_falsy("off"));

        assert!(!is_falsy("y"));
    }
}
