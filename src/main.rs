use reqwest::{Client, Url};
use std::collections::HashMap;
use std::{env, process};
use tokio::prelude::*;

mod cli;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// TODO: Use a `localhost` server to respond to this and display the `code`
const REDIRECT_URL: &str = "http://example.com";

// Exchange a client id/secret for an access token
async fn authorize(client_id: &str, secret: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();

    let params = vec![
        ("client_id", client_id),
        ("redirect_uri", REDIRECT_URL),
        ("response_type", "code"),
        (
            "scope",
            "https://www.googleapis.com/auth/photoslibrary.readonly",
        ),
        ("access_type", "offline"),
        ("state", "random"), // TODO: Randomly-generate a `state` parameter
        ("include_granted_scopes", "true"),
        ("prompt", "consent")
    ];

    let url = Url::parse_with_params(AUTH_URL, &params)?;

    println!("Open this URL in your browser:");
    println!("{}", url);

    let token = cli::read_cli_input("And enter the access token below:");

    let params = vec![
        ("client_id", client_id),
        ("redirect_uri", REDIRECT_URL),
        ("client_secret", secret),
        ("code", token.as_ref()),
        ("grant_type", "authorization_code"),
    ];

    let foo = client.post(TOKEN_URL).form(&params).send().await?;
    println!("{}", foo.text().await?);

    return Ok(String::from("foo"));
}

fn fail(message: &str) {
    println!("{}", message);
    process::exit(1)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env: HashMap<String, String> = env::vars().collect();

    let client_id = env
        .get("FERROTYPE_CLIENT_ID")
        .expect("Didn't get a FERROTYPE_CLIENT_ID");
    let secret = env
        .get("FERROTYPE_SECRET")
        .expect("Didn't get a FERROTYPE_SECRET");

    authorize(client_id, secret).await;

    Ok(())

    // 1. scan fs to determine local state of the world
    // 2. download metadata from the GP API to determine remote state of the world
    // 2.1 Auth
    // 3. diff 1. and 2. to determine what to download
    // 4. start downloading
    // 5. respect API limits
    // 6. do this once every X hours
}
