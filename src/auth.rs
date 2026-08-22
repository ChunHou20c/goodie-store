//! Accounts, roles and session tokens.
//!
//! A session is an opaque 256-bit token in an `HttpOnly` cookie; the database
//! stores only its SHA-256, so a dump of `sessions` cannot be replayed. Both the
//! page renderer and the server-function handler get `Parts` and
//! `ResponseOptions` in context from `leptos_axum`, so reading and writing that
//! cookie works the same during SSR as it does in a server function — no axum
//! middleware involved.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// How long a session lives, in the cookie and in the database.
pub const SESSION_MAX_AGE_SECS: i64 = 60 * 60 * 24 * 30;
pub const COOKIE_NAME: &str = "kessel_session";
pub const MIN_PASSWORD_LEN: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[cfg_attr(
    feature = "ssr",
    sqlx(type_name = "user_role", rename_all = "lowercase")
)]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::User => "Customer",
            Role::Admin => "Admin",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct AuthUser {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub role: Role,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    /// What the top bar and the account screen call you.
    pub fn name(&self) -> &str {
        match &self.display_name {
            Some(name) if !name.trim().is_empty() => name,
            _ => self.email.split('@').next().unwrap_or(&self.email),
        }
    }

    pub fn initial(&self) -> String {
        self.name()
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string())
    }
}

#[cfg(feature = "ssr")]
pub use ssr::{bootstrap_admin, current_user_from_request, require_admin, require_user};

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;

    use argon2::password_hash::rand_core::{OsRng, RngCore};
    use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
    use argon2::Argon2;
    use axum::http::header::{COOKIE, SET_COOKIE};
    use axum::http::request::Parts;
    use axum::http::HeaderValue;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use leptos_axum::ResponseOptions;
    use sha2::{Digest, Sha256};
    use sqlx::PgPool;

    pub fn pool() -> Result<PgPool, ServerFnError> {
        use_context::<PgPool>().ok_or_else(|| ServerFnError::new("no database pool in context"))
    }

    // ── passwords ──────────────────────────────────────────────────────────

    pub fn hash_password(password: &str) -> Result<String, ServerFnError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| ServerFnError::new(format!("could not hash password: {e}")))
    }

    pub fn verify_password(password: &str, phc: &str) -> bool {
        match PasswordHash::new(phc) {
            Ok(parsed) => Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok(),
            Err(_) => false,
        }
    }

    // ── cookies ────────────────────────────────────────────────────────────

    /// Pull one cookie out of a `Cookie:` header value.
    pub fn cookie_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
        header.split(';').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key.trim() == name).then(|| value.trim())
        })
    }

    pub fn read_token() -> Option<String> {
        let parts = use_context::<Parts>()?;
        let header = parts.headers.get(COOKIE)?.to_str().ok()?;
        cookie_value(header, COOKIE_NAME).map(str::to_owned)
    }

    fn write_cookie(value: String) {
        if let Some(response) = use_context::<ResponseOptions>() {
            if let Ok(header) = HeaderValue::from_str(&value) {
                response.append_header(SET_COOKIE, header);
            }
        }
    }

    /// `Secure` is opt-in so that plain-http localhost still works in dev.
    fn secure_attr() -> &'static str {
        match std::env::var("APP_SECURE_COOKIES").as_deref() {
            Ok("1") => "; Secure",
            _ => "",
        }
    }

    pub fn set_session_cookie(token: &str) {
        write_cookie(format!(
            "{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_MAX_AGE_SECS}{}",
            secure_attr()
        ));
    }

    pub fn clear_session_cookie() {
        write_cookie(format!(
            "{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
            secure_attr()
        ));
    }

    // ── sessions ───────────────────────────────────────────────────────────

    pub fn token_hash(token: &str) -> String {
        let digest = Sha256::digest(token.as_bytes());
        URL_SAFE_NO_PAD.encode(digest)
    }

    /// Opens a session and returns the plaintext token — the only time it exists
    /// outside the browser.
    pub async fn create_session(pool: &PgPool, user_id: i64) -> Result<String, sqlx::Error> {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token = URL_SAFE_NO_PAD.encode(bytes);

        sqlx::query(
            "insert into sessions (token_hash, user_id, expires_at) \
             values ($1, $2, now() + make_interval(secs => $3))",
        )
        .bind(token_hash(&token))
        .bind(user_id)
        .bind(SESSION_MAX_AGE_SECS as f64)
        .execute(pool)
        .await?;

        Ok(token)
    }

    /// The signed-in user for this request, if the cookie names a live session.
    pub async fn current_user_from_request() -> Option<AuthUser> {
        let token = read_token()?;
        let pool = pool().ok()?;
        sqlx::query_as::<_, AuthUser>(
            "select u.id, u.email, u.display_name, u.role \
             from sessions s join users u on u.id = s.user_id \
             where s.token_hash = $1 and s.expires_at > now()",
        )
        .bind(token_hash(&token))
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
    }

    pub async fn require_user() -> Result<AuthUser, ServerFnError> {
        current_user_from_request()
            .await
            .ok_or_else(|| ServerFnError::new("You need to be signed in to do that."))
    }

    /// The privilege boundary. Every admin-only server function starts here —
    /// hiding the UI is not authorization.
    pub async fn require_admin() -> Result<AuthUser, ServerFnError> {
        let user = require_user().await?;
        if user.is_admin() {
            Ok(user)
        } else {
            Err(ServerFnError::new("That is an admin-only action."))
        }
    }

    // ── first admin ────────────────────────────────────────────────────────

    /// Upserts the admin named by `ADMIN_EMAIL` / `ADMIN_PASSWORD`. Called at
    /// startup; does nothing unless both are set.
    pub async fn bootstrap_admin(pool: &PgPool) -> Result<Option<String>, ServerFnError> {
        let (Ok(email), Ok(password)) = (
            std::env::var("ADMIN_EMAIL"),
            std::env::var("ADMIN_PASSWORD"),
        ) else {
            return Ok(None);
        };
        if email.trim().is_empty() || password.is_empty() {
            return Ok(None);
        }

        let email = email.trim().to_lowercase();
        let hash = hash_password(&password)?;
        sqlx::query(
            "insert into users (email, password_hash, display_name, role) \
             values ($1, $2, $3, 'admin') \
             on conflict (email) do update \
             set password_hash = excluded.password_hash, role = 'admin'",
        )
        .bind(&email)
        .bind(&hash)
        .bind("Admin")
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(format!("could not bootstrap admin: {e}")))?;

        Ok(Some(email))
    }
}

// ── server functions ───────────────────────────────────────────────────────

/// Register and sign in, in one step.
#[server(endpoint = "sign_up")]
pub async fn sign_up(
    email: String,
    password: String,
    display_name: String,
) -> Result<(), ServerFnError> {
    use self::ssr::*;

    let email = email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 3 {
        return Err(ServerFnError::new(
            "That does not look like an email address.",
        ));
    }
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(ServerFnError::new(format!(
            "Passwords need at least {MIN_PASSWORD_LEN} characters."
        )));
    }

    let pool = pool()?;
    let display_name = display_name.trim();
    let display_name = (!display_name.is_empty()).then(|| display_name.to_string());

    let hash = hash_password(&password)?;
    let user_id: Option<(i64,)> = sqlx::query_as(
        "insert into users (email, password_hash, display_name) values ($1, $2, $3) \
         on conflict (email) do nothing returning id",
    )
    .bind(&email)
    .bind(&hash)
    .bind(&display_name)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("could not create the account: {e}")))?;

    let Some((user_id,)) = user_id else {
        return Err(ServerFnError::new("That email already has an account."));
    };

    let token = create_session(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(format!("could not start a session: {e}")))?;
    set_session_cookie(&token);
    Ok(())
}

#[server(endpoint = "sign_in")]
pub async fn sign_in(email: String, password: String) -> Result<(), ServerFnError> {
    use self::ssr::*;

    let email = email.trim().to_lowercase();
    let pool = pool()?;

    let found: Option<(i64, String)> =
        sqlx::query_as("select id, password_hash from users where email = $1")
            .bind(&email)
            .fetch_optional(&pool)
            .await
            .map_err(|e| ServerFnError::new(format!("sign-in failed: {e}")))?;

    // One message for both a missing account and a wrong password.
    let wrong = || ServerFnError::new("Email or password is wrong.");
    let Some((user_id, hash)) = found else {
        return Err(wrong());
    };
    if !verify_password(&password, &hash) {
        return Err(wrong());
    }

    let token = create_session(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(format!("could not start a session: {e}")))?;
    set_session_cookie(&token);
    Ok(())
}

#[server(endpoint = "sign_out")]
pub async fn sign_out() -> Result<(), ServerFnError> {
    use self::ssr::*;

    if let (Some(token), Ok(pool)) = (read_token(), pool()) {
        let _ = sqlx::query("delete from sessions where token_hash = $1")
            .bind(token_hash(&token))
            .execute(&pool)
            .await;
    }
    clear_session_cookie();
    Ok(())
}

#[server(endpoint = "current_user")]
pub async fn current_user() -> Result<Option<AuthUser>, ServerFnError> {
    Ok(self::ssr::current_user_from_request().await)
}

// ── reactive context ───────────────────────────────────────────────────────

/// Who is signed in, plus the actions that change that.
///
/// Same shape as [`crate::catalog::Catalog`]: a blocking resource resolved
/// during SSR and read from the serialized response on the client. The extra
/// piece is the source — it tracks the three actions' versions, so signing in or
/// out reloads the user without a page load.
#[derive(Clone, Copy)]
pub struct Auth {
    user: Resource<Result<Option<AuthUser>, ServerFnError>>,
    pub sign_in: ServerAction<SignIn>,
    pub sign_up: ServerAction<SignUp>,
    pub sign_out: ServerAction<SignOut>,
}

impl Auth {
    pub fn load() -> Self {
        let sign_in = ServerAction::<SignIn>::new();
        let sign_up = ServerAction::<SignUp>::new();
        let sign_out = ServerAction::<SignOut>::new();

        let version = move || {
            (
                sign_in.version().get(),
                sign_up.version().get(),
                sign_out.version().get(),
            )
        };
        let user = Resource::new_blocking(version, |_| current_user());

        Self {
            user,
            sign_in,
            sign_up,
            sign_out,
        }
    }

    pub fn from_context() -> Self {
        expect_context()
    }

    /// Read the signed-in user without cloning. A failed lookup reads as signed
    /// out — the chrome should never break because the session query did.
    pub fn with<T>(&self, f: impl FnOnce(Option<&AuthUser>) -> T) -> T {
        self.user.with(|loaded| match loaded {
            Some(Ok(Some(user))) => f(Some(user)),
            _ => f(None),
        })
    }

    pub fn user(&self) -> Option<AuthUser> {
        self.with(|user| user.cloned())
    }

    pub fn is_signed_in(&self) -> bool {
        self.with(|user| user.is_some())
    }

    pub fn is_admin(&self) -> bool {
        self.with(|user| user.is_some_and(AuthUser::is_admin))
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::ssr::{cookie_value, hash_password, token_hash, verify_password};
    use super::COOKIE_NAME;

    #[test]
    fn password_round_trips() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("Correct horse battery", &hash));
        assert!(!verify_password("", &hash));
        // Same password, different salt, different PHC string.
        assert_ne!(hash, hash_password("correct horse battery").unwrap());
    }

    #[test]
    fn garbage_hashes_do_not_authenticate() {
        assert!(!verify_password("anything", "not-a-phc-string"));
    }

    #[test]
    fn finds_the_session_cookie_among_others() {
        let name = COOKIE_NAME;
        assert_eq!(cookie_value(&format!("{name}=abc"), name), Some("abc"));
        assert_eq!(
            cookie_value(&format!("theme=dark; {name}=abc; other=1"), name),
            Some("abc")
        );
        // Servers send `; ` separated pairs; tolerate stray whitespace.
        assert_eq!(cookie_value(&format!("a=1;{name}=abc"), name), Some("abc"));
        assert_eq!(cookie_value("theme=dark", name), None);
        assert_eq!(cookie_value("", name), None);
        // A cookie whose name merely ends with ours must not match.
        assert_eq!(cookie_value(&format!("not_{name}=abc"), name), None);
    }

    #[test]
    fn token_hash_is_stable_and_not_the_token() {
        let token = "a-token";
        assert_eq!(token_hash(token), token_hash(token));
        assert_ne!(token_hash(token), token);
        assert_ne!(token_hash(token), token_hash("b-token"));
    }
}
