use derive_getters::Getters;

use super::Role;

#[derive(Debug, Clone, Getters)]
#[allow(dead_code)]
pub struct Policy {
    user_id: tachyon_sdk::auth::UserId,
    role: Role,
}

impl Policy {
    #[allow(dead_code)]
    pub fn new(user_id: &tachyon_sdk::auth::UserId, role: Role) -> Self {
        Self {
            user_id: user_id.clone(),
            role,
        }
    }
}
