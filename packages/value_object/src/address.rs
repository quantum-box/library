use derive_getters::Getters;
use std::{
    fmt::{self, Display},
    str::FromStr,
};

// TODO: add English comment
#[derive(Clone, Debug, PartialEq, Eq, Hash, Getters)]
pub struct Address {
    postal_code: String,
    state: String,
    city: String,
    address1: String,
    address2: Option<String>,
}

impl Address {
    pub fn new(
        postal_code: &str,
        state: &str,
        city: &str,
        address1: &str,
        address2: Option<&str>,
    ) -> errors::Result<Self> {
        // TODO: add English comment
        if postal_code.contains('_')
            || state.contains('_')
            || city.contains('_')
            || address1.contains('_')
            || address2.is_some_and(|a| a.contains('_'))
        {
            return Err(errors::Error::business_logic(
                "住所にアンダースコアを含めることはできません",
            ));
        }
        if postal_code.is_empty() {
            return Err(errors::Error::business_logic(
                "郵便番号は必須です",
            ));
        }
        if state.is_empty() {
            return Err(errors::Error::business_logic(
                "都道府県は必須です",
            ));
        }
        if city.is_empty() {
            return Err(errors::Error::business_logic("市区は必須です"));
        }
        if address1.is_empty() {
            return Err(errors::Error::business_logic("住所1は必須です"));
        }

        Ok(Self {
            postal_code: postal_code.to_string(),
            state: state.to_string(),
            city: city.to_string(),
            address1: address1.to_string(),
            address2: address2.map(|s| s.to_string()),
        })
    }
}

impl FromStr for Address {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('_').collect::<Vec<&str>>();
        if parts.len() < 4 {
            return Err(errors::Error::business_logic(
                "Parse address error. Address must be in the format of '123-4567_Tokyo_Chiyoda_1-1-1_Bldg100'",
            ));
        }
        let postal_code = parts[0];
        let state = parts[1];
        let city = parts[2];
        let address1 = parts[3];
        let address2 = parts[4];
        if address2.is_empty() {
            Self::new(postal_code, state, city, address1, None)
        } else {
            Self::new(postal_code, state, city, address1, Some(address2))
        }
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}_{}_{}_{}_{}",
            self.postal_code,
            self.state,
            self.city,
            self.address1,
            self.address2.as_deref().unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_is_valid() {
        let address = Address::new(
            "100-0000",
            "東京都",
            "千代田区",
            "千代田1-1-1",
            None,
        )
        .unwrap();
        assert_eq!(
            address.to_string(),
            "100-0000_東京都_千代田区_千代田1-1-1_"
        );
    }

    #[test]
    fn test_address2_is_none_parsing() {
        assert_eq!(
            Address::new(
                "100-0000",
                "東京都",
                "千代田区",
                "千代田1-1-1",
                None,
            )
            .unwrap(),
            "100-0000_東京都_千代田区_千代田1-1-1_"
                .parse::<Address>()
                .unwrap()
        );
    }

    #[test]
    fn test_address2_is_some_parsing() {
        assert_eq!(
            Address::new(
                "100-0000",
                "東京都",
                "千代田区",
                "千代田1-1-1",
                Some("ビル100"),
            )
            .unwrap(),
            "100-0000_東京都_千代田区_千代田1-1-1_ビル100"
                .parse::<Address>()
                .unwrap()
        );
    }

    #[test]
    fn test_new_is_invalid() {
        assert!(Address::new(
            "100-0000",
            "東京都",
            "千代田区",
            "千代田1-1-1_",
            Some("ビル100")
        )
        .is_err(),);
    }
}
