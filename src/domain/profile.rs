#[derive(Debug, PartialEq)]
pub struct Profile {
    id: i32,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
}

pub fn new_profile(id: i32, username: String, email: String, first_name: String, last_name: String) -> Profile {
    Profile {
        id,
        username,
        email,
        first_name,
        last_name
    }
}

pub struct CreateProfileCommand {
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}