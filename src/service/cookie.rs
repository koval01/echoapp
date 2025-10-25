use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration;

pub struct CookieService;

impl CookieService {
    pub fn create_auth_cookie(token: &str, max_age_hours: i64) -> Cookie<'static> {
        Cookie::build(("__Host-auth_token", token.to_string()))
            .http_only(true)
            .secure(true)
            .max_age(Duration::hours(max_age_hours))
            .path("/")
            .same_site(SameSite::Lax)
            .into()
    }

    pub fn add_auth_cookie(jar: CookieJar, token: &str, max_age_hours: i64) -> CookieJar {
        let cookie = Self::create_auth_cookie(token, max_age_hours);
        jar.add(cookie)
    }
}
