use std::env;

use crate::models::personality_config::PersonalityConfig;
use crate::models::token_model::Claims;
use crate::models::user::User;
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

use super::{ADMIN_ROLE, BUILDSERVER_ROLE};

fn generate_access_token(role: String, token_secret: &String) -> anyhow::Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(60))
        .expect("Invalid timestamp")
        .timestamp();

    let claims = Claims {
        role,
        exp: expiration as usize,
    };

    let header = Header::new(Algorithm::HS512);

    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(token_secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_access_token(token: String) -> anyhow::Result<Claims> {
    dotenv::dotenv().expect("Failed to load .env file");
    let token_secret =
        env::var("TOKEN_SECRET").expect("Environment variable 'TOKEN_SECRET' not found");

    println!("{:?}", token);
    let token_value = token.trim_start_matches("Bearer").trim();

    let claim = decode::<Claims>(
        token_value,
        &DecodingKey::from_secret(token_secret.as_bytes()),
        &Validation::new(Algorithm::HS512),
    )?
    .claims;
    Ok(claim)
}

#[post("/request-access-token", format = "application/json", data = "<user>")]
pub fn login(user: Json<User>, config: &State<PersonalityConfig>) -> Result<String, Status> {
    let user = user.into_inner();

    if user.name == ADMIN_ROLE && user.password == config.admin_password {
        let token = generate_access_token(String::from(ADMIN_ROLE), &config.token_secret)
            .map_err(|_| rocket::http::Status::Unauthorized)?;
        return Ok(token);
    }
    if user.name == BUILDSERVER_ROLE && user.password == config.buildserver_password {
        let token = generate_access_token(String::from(BUILDSERVER_ROLE), &config.token_secret)
            .map_err(|_| rocket::http::Status::Unauthorized)?;
        return Ok(token);
    }
    Err(rocket::http::Status::Unauthorized)
}
