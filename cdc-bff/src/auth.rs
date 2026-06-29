use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::AppState;

pub struct OAuthProvider {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_url: String,
    pub userinfo_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub provider: String,
    pub exp: usize,
}

pub struct AuthState {
    pub jwt_secret: Vec<u8>,
    pub local_users: HashMap<String, String>,
    pub oauth_providers: HashMap<String, OAuthProvider>,
    pub pkce_verifiers: RwLock<HashMap<String, PkceCodeVerifier>>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct OAuthCallbackParams {
    pub code: String,
    pub state: String,
}

#[derive(Serialize)]
pub struct AuthTokenResponse {
    pub token: String,
    pub user: String,
}

pub async fn login_local(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthTokenResponse>, StatusCode> {
    let auth_state = &state.auth_state;
    let stored_hash = auth_state
        .local_users
        .get(&payload.username)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if bcrypt::verify(&payload.password, stored_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let token = generate_jwt(&auth_state.jwt_secret, &payload.username, "local")?;
        Ok(Json(AuthTokenResponse {
            token,
            user: payload.username,
        }))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn oauth_login(
    State(state): State<Arc<AppState>>,
    Path(provider_name): Path<String>,
) -> Result<Redirect, StatusCode> {
    let auth_state = &state.auth_state;
    let provider = auth_state
        .oauth_providers
        .get(&provider_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    let client = BasicClient::new(ClientId::new(provider.client_id.clone()))
        .set_client_secret(ClientSecret::new(provider.client_secret.clone()))
        .set_auth_uri(
            AuthUrl::new(provider.auth_url.clone())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .set_token_uri(
            TokenUrl::new(provider.token_url.clone())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .set_redirect_uri(
            RedirectUrl::new(provider.redirect_url.clone())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    auth_state
        .pkce_verifiers
        .write()
        .await
        .insert(csrf_state.secret().clone(), pkce_verifier);
    Ok(Redirect::temporary(auth_url.as_str()))
}

pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Path(provider_name): Path<String>,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<Json<AuthTokenResponse>, StatusCode> {
    let auth_state = &state.auth_state;
    let provider = auth_state
        .oauth_providers
        .get(&provider_name)
        .ok_or(StatusCode::NOT_FOUND)?;
    let pkce_verifier = auth_state
        .pkce_verifiers
        .write()
        .await
        .remove(&params.state)
        .ok_or(StatusCode::BAD_REQUEST)?;

    let client = BasicClient::new(ClientId::new(provider.client_id.clone()))
        .set_client_secret(ClientSecret::new(provider.client_secret.clone()))
        .set_auth_uri(
            AuthUrl::new(provider.auth_url.clone())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .set_token_uri(
            TokenUrl::new(provider.token_url.clone())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .set_redirect_uri(
            RedirectUrl::new(provider.redirect_url.clone())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );
    let http_client = reqwest::Client::new();
    let token_result = client
        .exchange_code(AuthorizationCode::new(params.code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let access_token = token_result.access_token().secret();

    let user_info: serde_json::Value = http_client
        .get(&provider.userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let username: String = user_info["login"]
        .as_str()
        .or_else(|| user_info["preferred_username"].as_str())
        .or_else(|| user_info["email"].as_str())
        .or_else(|| user_info["name"].as_str())
        .or_else(|| user_info["sub"].as_str())
        .unwrap_or("unknown_user")
        .to_string();

    let token = generate_jwt(&auth_state.jwt_secret, &username, &provider_name)?;
    Ok(Json(AuthTokenResponse {
        token,
        user: username,
    }))
}

fn generate_jwt(secret: &[u8], sub: &str, provider: &str) -> Result<String, StatusCode> {
    let now_secs = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    };
    let exp_u64 = now_secs.saturating_add(86_400);

    let claims = Claims {
        sub: sub.to_string(),
        provider: provider.to_string(),
        exp: usize::try_from(exp_u64).unwrap_or(usize::MAX),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let token = auth.token();
    let validation = Validation::default();
    let secret = state.auth_state.jwt_secret.as_slice();
    match decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation) {
        Ok(_) => next.run(req).await,
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}
