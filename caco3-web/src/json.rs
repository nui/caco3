use std::borrow::Cow;
use std::marker::PhantomData;

use serde::{Serialize, Serializer};

type StrCow = Cow<'static, str>;

const DEFAULT_SUCCESS_CODE: &str = "0";
const DEFAULT_ERROR_CODE: &str = "-1";
const DEFAULT_ERROR_MESSAGE: &str = "Internal server error";

#[derive(Default)]
pub struct ApiJsonErrorBuilder<T> {
    code: Option<StrCow>,
    error: Option<StrCow>,
    _phantom: PhantomData<T>,
}

impl<T: Serialize> ApiJsonErrorBuilder<T> {
    pub fn new() -> Self {
        Self {
            code: None,
            error: None,
            _phantom: PhantomData,
        }
    }

    pub fn code(mut self, code: impl Into<StrCow>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn error(mut self, error: impl Into<StrCow>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn build(self) -> ApiJson<T> {
        ApiJson::Error {
            code: self.code.or_else(|| Some(DEFAULT_ERROR_CODE.into())),
            error: self.error.or_else(|| Some(DEFAULT_ERROR_MESSAGE.into())),
        }
    }
}

/// Standard Api response formatter.
pub enum ApiJson<T> {
    Data {
        code: Option<StrCow>,
        data: Option<T>,
    },
    Error {
        code: Option<StrCow>,
        error: Option<StrCow>,
    },
}

impl<T: Serialize> ApiJson<T> {
    pub fn ok(data: T) -> Self {
        Self::Data {
            data: Some(data),
            code: None,
        }
    }

    pub fn data_with_code(data: T, code: StrCow) -> Self {
        Self::Data {
            data: Some(data),
            code: Some(code),
        }
    }

    pub fn error_builder() -> ApiJsonErrorBuilder<T> {
        ApiJsonErrorBuilder::<T>::new()
    }

    fn as_serializable(&self) -> ApiJsonSerializable<'_, T> {
        match *self {
            Self::Data { ref code, ref data } => ApiJsonSerializable {
                code: code.as_deref().unwrap_or(DEFAULT_SUCCESS_CODE),
                error: None,
                data: data.as_ref(),
            },
            Self::Error {
                ref code,
                ref error,
            } => ApiJsonSerializable {
                code: code.as_deref().unwrap_or(DEFAULT_ERROR_CODE),
                error: error.as_deref(),
                data: None,
            },
        }
    }
}

impl ApiJson<()> {
    /// Convenience method to build error without specifying generic type parameter
    pub fn unit_error_builder() -> ApiJsonErrorBuilder<()> {
        ApiJsonErrorBuilder::new()
    }

    pub fn no_content() -> Self {
        Self::Data {
            data: Some(()),
            code: None,
        }
    }

    pub const fn default_error() -> Self {
        Self::Error {
            code: Some(StrCow::Borrowed(DEFAULT_ERROR_CODE)),
            error: Some(StrCow::Borrowed(DEFAULT_ERROR_MESSAGE)),
        }
    }
}

impl<T: Serialize> Serialize for ApiJson<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        Serialize::serialize(&self.as_serializable(), serializer)
    }
}

#[derive(Serialize)]
struct ApiJsonSerializable<'a, T> {
    code: &'a str,
    #[serde(rename = "data", skip_serializing_if = "Option::is_none")]
    data: Option<&'a T>,
    #[serde(rename = "message", skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum JsonMode {
    Normal,
    Pretty,
}

impl JsonMode {
    pub fn to_string<T>(self, value: &T) -> serde_json::Result<String>
        where
            T: ?Sized + Serialize,
    {
        match self {
            Self::Normal => serde_json::to_string(value),
            Self::Pretty => serde_json::to_string_pretty(value),
        }
    }

    pub fn to_vec<T>(self, value: &T) -> serde_json::Result<Vec<u8>>
        where
            T: ?Sized + Serialize,
    {
        match self {
            Self::Normal => serde_json::to_vec(value),
            Self::Pretty => serde_json::to_vec_pretty(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[derive(Serialize, Clone)]
    struct TestData {
        foo: String,
    }

    #[test]
    fn test_ok_data() {
        let data = TestData {
            foo: "bar".to_owned(),
        };

        let json = ApiJson::ok(data);
        let actual = serde_json::to_value(json).unwrap();

        let expect = json!({
            "code": DEFAULT_SUCCESS_CODE,
            "data": {
                "foo": "bar"
            },
        });

        assert_eq!(actual, expect);
    }

    #[test]
    fn test_ok_data_with_code() {
        let data = TestData {
            foo: "bar".to_owned(),
        };

        let json = ApiJson::data_with_code(data, "Nani!".into());
        let actual = serde_json::to_value(json).unwrap();

        let expect = json!({
            "code": "Nani!",
            "data": {
                "foo": "bar"
            },
        });

        assert_eq!(actual, expect);
    }

    #[test]
    fn test_no_content() {
        let json = ApiJson::no_content();
        let actual = serde_json::to_value(json).unwrap();

        let expect = json!({
            "code": DEFAULT_SUCCESS_CODE,
            "data": serde_json::Value::Null,
        });

        assert_eq!(actual, expect);
    }

    #[test]
    fn test_build_default_error_message() {
        let json = ApiJson::unit_error_builder().build();
        let actual = serde_json::to_value(json).unwrap();
        let expect = json!({
            "code": DEFAULT_ERROR_CODE,
            "message": DEFAULT_ERROR_MESSAGE,
        });
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_build_error_with_message() {
        let json = ApiJson::unit_error_builder().error("foo").build();
        let actual = serde_json::to_value(json).unwrap();
        let expect = json!({
            "code": DEFAULT_ERROR_CODE,
            "message": "foo",
        });
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_build_error_with_code_and_message() {
        let json = ApiJson::unit_error_builder()
            .code("-1")
            .error("bar")
            .build();
        let actual = serde_json::to_value(json).unwrap();
        let expect = json!({
            "code": "-1",
            "message": "bar",
        });
        assert_eq!(actual, expect);
    }

    #[test]
    fn test_api_json_is_send_and_sync() {
        fn require_send(_: impl Send + Sync) {}
        require_send(ApiJson::ok(()));
    }
}
