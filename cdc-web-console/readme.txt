For production, replace localStorage JWT storage with HttpOnly cookies set by the BFF to prevent XSS attacks. 
Modify the BFF's auth.rs to use axum-extra's CookieJar instead of returning the JWT in the JSON body.

export GITHUB_CLIENT_ID="your_github_client_id"
export GITHUB_CLIENT_SECRET="your_github_client_secret"
export GITHUB_REDIRECT_URL="http://localhost:8080/api/auth/oauth2/github/callback"