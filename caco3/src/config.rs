pub use bool_from_config_value::bool_from_config_value;
pub use meta::MetaConfig;

mod bool_from_config_value;
mod meta;

pub fn is_yes<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();
    ["1", "true", "y", "yes", "on"]
        .into_iter()
        .any(|s| value.eq_ignore_ascii_case(s))
}

pub fn is_no<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();
    ["0", "false", "n", "no", "off"]
        .into_iter()
        .any(|s| value.eq_ignore_ascii_case(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_yes() {
        assert!(is_yes("1"));
        assert!(is_yes("true"));
        assert!(is_yes("y"));
        assert!(is_yes("Y"));
        assert!(is_yes("yes"));
        assert!(is_yes("on"));

        assert!(!is_yes("n"));
    }

    #[test]
    fn test_is_no() {
        assert!(is_no("0"));
        assert!(is_no("false"));
        assert!(is_no("n"));
        assert!(is_no("N"));
        assert!(is_no("no"));
        assert!(is_no("off"));

        assert!(!is_no("y"));
    }
}
