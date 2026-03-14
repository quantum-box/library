use async_graphql::InputObject;
use errors::Error;

#[derive(InputObject, Debug)]
pub struct AddressInput {
    pub postal_code: String,
    pub state: String,
    pub city: String,
    pub address1: String,
    pub address2: Option<String>,
}

impl TryFrom<AddressInput> for crate::Address {
    type Error = Error;
    fn try_from(value: AddressInput) -> Result<Self, Self::Error> {
        crate::Address::new(
            &value.postal_code,
            &value.state,
            &value.city,
            &value.address1,
            value.address2.as_deref(),
        )
    }
}
