use jsonwebtoken::errors::{Error, ErrorKind};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome, Request},
    serde::{Deserialize, Serialize},
};

use crate::controllers::authentication_controller::verify_access_token;

use super::response_model::ApiResponse;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub role: String,
    pub exp: usize,
}

pub struct AccessToken {
    pub claims: Claims,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AccessToken {
    type Error = ApiResponse;
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ApiResponse> {
        fn is_valid(key: &str) -> Result<Claims, Error> {
            let result =
                verify_access_token(String::from(key)).map_err(|_| ErrorKind::InvalidToken)?;
            Ok(result)
        }

        match req.headers().get_one("Authorization") {
            None => Outcome::Error((
                Status::Unauthorized,
                ApiResponse::Unauthorized(String::from("No token found")),
            )),
            Some(key) => match is_valid(key) {
                Ok(claims) => Outcome::Success(AccessToken { claims }),
                Err(_err) => Outcome::Error((
                    Status::Unauthorized,
                    ApiResponse::Unauthorized(String::from("Token not valid")),
                )),
            },
        }
    }
}
