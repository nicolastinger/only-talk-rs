use common::models::user_entity::basic_user::BasicUser;
use common::models::user_entity::user_info::UserInfo;
use common::utils::time::get_now_time_stamp_as_millis;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Deserialize, Serialize, Debug, Validate)]
pub struct UpdateUserDTO {
    #[validate(length(max = 50, message = "用户名长度不能超过50"))]
    pub username: Option<String>,
    #[validate(length(max = 200, message = "简介长度不能超过200"))]
    pub info: Option<String>,
    pub gender: Option<u8>,
    pub age: Option<u8>,
    pub birthday: Option<i64>,
    #[validate(phone(message = "手机号格式不正确"))]
    pub phone: Option<String>,
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(max = 200, message = "地址长度不能超过200"))]
    pub address: Option<String>,
}

impl UpdateUserDTO {
    pub fn apply_to_basic_user(&self, basic_user: &mut BasicUser) {
        if let Some(ref username) = self.username {
            basic_user.username = Some(username.clone());
        }
        if let Some(ref info) = self.info {
            basic_user.info = Some(info.clone());
        }
    }

    pub fn apply_to_user_info(&self, user_info: &mut UserInfo) -> Result<(), anyhow::Error> {
        if self.gender.is_some() {
            user_info.gender = self.gender;
        }
        if self.age.is_some() {
            user_info.age = self.age;
        }
        if self.birthday.is_some() {
            user_info.birthday = self.birthday;
        }
        if let Some(ref phone) = self.phone {
            user_info.phone = Some(phone.clone());
        }
        if let Some(ref email) = self.email {
            user_info.email = Some(email.clone());
        }
        if let Some(ref address) = self.address {
            user_info.address = Some(address.clone());
        }
        user_info.updated_at = Some(get_now_time_stamp_as_millis()?);
        Ok(())
    }
}
