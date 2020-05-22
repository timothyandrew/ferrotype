use reqwest::{Client, Url};
use std::collections::HashMap;
use std::time::Instant;
use serde::Deserialize;

use crate::cli;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// TODO: Use a `localhost` server to respond to this and display the `code`
const REDIRECT_URL: &str = "http://example.com";

pub struct Credentials {
  access_token: String,
  refresh_token: String,
  validity: i32,
  created_at: Instant
}

impl Credentials {
  pub fn get_key(&self) -> &str {
    &self.access_token
  }

  pub fn dummy(key: &str) -> Credentials {
    Credentials {
      access_token: String::from(key),
      refresh_token: String::from(""),
      validity: 0,
      created_at: Instant::now()
    }
  }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AuthResponse {
  access_token: String,
  refresh_token: String,
  expires_in: i32,
  scope: String,
  token_type: String
}

// Exchange a client id/secret for an access token
pub async fn authorize(client_id: &str, secret: &str) -> Result<Credentials, Box<dyn std::error::Error>> {
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
        ("prompt", "consent"),
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

    let response = client.post(TOKEN_URL).form(&params).send().await?;
    let AuthResponse {access_token, refresh_token, expires_in, ..} = response.json::<AuthResponse>().await?;

    // TODO: Remove this
    println!("Got access token: {}", access_token);

    let credentials = Credentials { access_token, refresh_token, validity: expires_in, created_at: Instant::now()  };

    Ok(credentials)
}