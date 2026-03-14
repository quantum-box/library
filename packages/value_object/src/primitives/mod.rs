// TODO: add English comment

mod provider_name;
pub use provider_name::*;

use serde::{Deserialize, Serialize};

/// # Text
/// TODO: add English documentation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct Text(String);

impl Text {
    const MAX_LENGTH: usize = 191;

    pub fn new(text: &str) -> anyhow::Result<Self> {
        if text.trim().is_empty() {
            return Err(anyhow::anyhow!(
                t!("err.empty_type", typ => "Text")
            ));
        }
        if text.len() > Self::MAX_LENGTH {
            return Err(anyhow::anyhow!(
                t!("err.too_long_type", typ => "Text", max => Self::MAX_LENGTH)
            ));
        }
        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)?;
        Ok(())
    }
}

impl From<String> for Text {
    fn from(val: String) -> Self {
        Text::new(&val).unwrap()
    }
}

impl std::convert::TryInto<Text> for &str {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Text, Self::Error> {
        Text::new(self)
    }
}

impl std::str::FromStr for Text {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

// impl std::convert::TryInto<Text> for String {
//     type Error = anyhow::Error;
//
//     fn try_into(self) -> Result<Text, Self::Error> {
//         Text::new(&self)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[derive(Serialize, Deserialize)]
    struct Sample {
        text: Text,
    }

    #[rstest]
    #[case("{\"text\":\"hello\"}", "hello")]
    fn test_text_serde(#[case] json: &str, #[case] text: &str) {
        let sample: Sample = serde_json::from_str(json).unwrap();
        assert_eq!(sample.text.clone(), Text::new(text).unwrap());

        let json = serde_json::to_string(&sample).unwrap();
        assert_eq!(json, "{\"text\":\"hello\"}");
    }
}

/// # LongText
/// TODO: add English documentation
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub struct LongText(String);

impl LongText {
    const MAX_LENGTH: usize = 1_000_000;

    pub fn new(text: &str) -> anyhow::Result<Self> {
        if text.trim().is_empty() {
            return Err(anyhow::anyhow!(
                t!("err.empty_type", typ => "LongText")
            ));
        }
        if text.len() > Self::MAX_LENGTH {
            return Err(anyhow::anyhow!(
                t!("err.too_long_type", typ => "LongText", max => Self::MAX_LENGTH)
            ));
        }
        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LongText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::convert::TryInto<LongText> for &str {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<LongText, Self::Error> {
        LongText::new(self)
    }
}

impl std::convert::TryInto<LongText> for String {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<LongText, Self::Error> {
        LongText::new(&self)
    }
}

impl std::str::FromStr for LongText {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
