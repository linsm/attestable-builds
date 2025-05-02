extern crate dotenv;
#[macro_use]
extern crate rocket;
mod controllers;
mod models;

#[allow(clippy::all)] // don't lint generated code
mod trillian_rust;

use crate::models::personality_config::PersonalityConfig;
use controllers::{
    admin_controller, authentication_controller, log_builder_controller, log_controller,
};
use std::env;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    dotenv::dotenv().expect("Failed to load .env file");
    let trillian_url =
        env::var("TRILLIAN_URL").expect("Environment variable 'TRILLIAN_URL' not found");
    let admin_password =
        env::var("ADMIN_PASSWORD").expect("Environment variable 'ADMIN_PASSWORD' not found");
    let buildserver_password = env::var("BUILDSERVER_PASSWORD")
        .expect("Environment variable 'BUILDSERVER_PASSWORD' not found");
    let token_secret =
        env::var("TOKEN_SECRET").expect("Environment variable 'TOKEN_SECRET' not found");

    println!("Trillian URL: {}", trillian_url);

    let personality_config = PersonalityConfig::new(
        trillian_url,
        admin_password,
        buildserver_password,
        token_secret,
    );

    rocket::build()
        .manage(personality_config)
        .mount(
            "/admin",
            routes![admin_controller::create_tree],
        )
        .mount(
            "/log",
            routes![
                log_controller::list_trees,
                log_controller::inclusion_proof,
                log_controller::latest_signed_log_root
            ],
        )
        .mount(
            "/logbuilder",
            routes![log_builder_controller::add_log_entry],
        )
        .mount("/login", routes![authentication_controller::login])
        .launch()
        .await?;
    Ok(())
}
