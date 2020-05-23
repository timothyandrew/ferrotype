use reqwest::{Client, Url};
use serde::Deserialize;

use std::time::Instant;

use crate::cli;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// TODO: Use a `localhost` server to respond to this and display the `code`
const REDIRECT_URL: &str = "http://example.com";

#[derive(Debug)]
pub struct Credentials {
    access_token: String,
    refresh_token: String,
    validity: u64,
    created_at: Instant,
    client_id: String,
    client_secret: String,
}

impl Credentials {
    pub fn get_key(&self) -> &str {
        &self.access_token
    }


    /// Instantiate a new `Credentials` instance with previously-received refresh token.
    /// This instance isn't usable until `refresh` is called on it (to exchange the refresh
    /// token for an access token).
    pub fn with_refresh_token(refresh_token: &str, client_id: &str, secret: &str) -> Credentials {
        Credentials {
            access_token: String::new(),
            refresh_token: refresh_token.to_string(),
            validity: 0,
            created_at: Instant::now(),
            client_id: client_id.to_string(),
            client_secret: secret.to_string()
        }
    }

    pub fn is_token_expiry_imminent(&self) -> bool {
        // TODO: Use Duration directly for this
        let buffer = 300; // seconds
        (Instant::now().duration_since(self.created_at).as_secs() + buffer) >= self.validity
    }

    pub async fn refresh(&self) -> Result<Credentials, Box<dyn std::error::Error>> {
        let client = Client::new();

        let params = vec![
            ("client_id", &self.client_id[..]),
            ("client_secret", &self.client_secret[..]),
            ("refresh_token", &self.refresh_token[..]),
            ("grant_type", "refresh_token"),
        ];

        // TODO: Handle non-200s
        let response = client.post(TOKEN_URL).form(&params).send().await?;
        let RefreshResponse {
            access_token,
            expires_in,
            ..
        } = response.json::<RefreshResponse>().await?;

        println!("Refreshed authorization with token: {}", access_token);

        let credentials = Credentials {
            access_token,
            refresh_token: self.refresh_token.clone(),
            validity: expires_in,
            created_at: Instant::now(),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
        };

        Ok(credentials)
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    scope: String,
    token_type: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct RefreshResponse {
    access_token: String,
    expires_in: u64,
    scope: String,
    token_type: String,
}

// Exchange a client id/secret for an access token
pub async fn authorize(
    client_id: &str,
    secret: &str,
) -> Result<Credentials, Box<dyn std::error::Error>> {
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

    // TODO: Handle non-200s
    let response = client.post(TOKEN_URL).form(&params).send().await?;
    let AuthResponse {
        access_token,
        refresh_token,
        expires_in,
        ..
    } = response.json::<AuthResponse>().await?;

    let credentials = Credentials {
        access_token,
        refresh_token,
        validity: expires_in,
        created_at: Instant::now(),
        client_id: client_id.to_string(),
        client_secret: secret.to_string(),
    };

    println!("Received credentials: {:?}", credentials);

    Ok(credentials)
}
