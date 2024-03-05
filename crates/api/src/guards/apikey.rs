use oxidized_config::get_config;
use rocket::{http::Method, request::FromRequest};

pub struct ApiKeyGuard;

#[async_trait::async_trait]
impl<'r> FromRequest<'r> for ApiKeyGuard {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let config = get_config();

        let api_key = match request.method() {
            Method::Get => request
                .query_value("apikey")
                .map(|x| x.unwrap_or(""))
                .map(|x| x.to_string()),
            Method::Post => request
                .headers()
                .get_one("Authorization")
                .map(|x| x.replace("Bearer ", "").to_string()),
            _ => None,
        };

        if api_key.unwrap_or("".to_string()) == config.auth.apikey.unwrap_or("".to_string()) {
            rocket::request::Outcome::Success(ApiKeyGuard)
        } else {
            rocket::request::Outcome::Error((rocket::http::Status::Unauthorized, ()))
        }
    }
}
