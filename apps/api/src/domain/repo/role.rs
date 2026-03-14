use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, Display)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Owner,
    // Maintainer,
    Writer,
    Reader,
}

impl Role {
    #[allow(dead_code)]
    pub fn can_write(&self) -> bool {
        match self {
            Role::Owner => true,
            // Role::Maintainer => true,
            Role::Writer => true,
            Role::Reader => false,
        }
    }
}
