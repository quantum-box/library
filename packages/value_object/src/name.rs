use derive_getters::Getters;
use errors::Error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Eq, PartialEq, Clone, Getters)]
pub struct Name {
    pub first: String,
    pub last: String,
}

impl Name {
    pub fn new(first: &str, last: &str) -> Self {
        Self {
            first: first.to_string(),
            last: last.to_string(),
        }
    }
    pub fn join(&self) -> String {
        format!("{} {}", self.first, self.last)
    }
}

impl FromStr for Name {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.replace('　', " "); // TODO: add English comment
        let split: Vec<&str> = s.split_whitespace().collect();
        if split.len() != 2 {
            return Err(Error::invalid(
                "Name must be two words separated by a space",
            ));
        }
        Ok(Self {
            first: split[0].to_string(),
            last: split[1].to_string(),
        })
    }
}

impl From<String> for Name {
    fn from(value: String) -> Self {
        let split: Vec<&str> = value.split_whitespace().collect();
        Self {
            first: split[0].to_string(),
            last: split[1].to_string(),
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.first, self.last)
    }
}
