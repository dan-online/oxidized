use config::{Config, Environment, File};
use serde::Serialize;
use serde_derive::Deserialize;

#[derive(Deserialize, Serialize)]
pub struct AuthSettings {
    pub apikey: String,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DatabaseSettings {
    pub url: String,
    pub sqlx_logging: bool,
}

#[derive(Deserialize, Serialize)]
pub struct AppSettings {
    pub spider: bool,
}

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app: AppSettings,
    pub auth: AuthSettings,
}

pub fn get_config() -> Settings {
    let settings = Config::builder()
        .add_source(File::with_name("config/default"))
        .add_source(File::with_name("~/.config/oxidized").required(false))
        .add_source(
            Environment::with_prefix("OXIDIZED")
                .separator("_")
                .try_parsing(true),
        )
        .build()
        .unwrap();

    settings.try_deserialize().unwrap()
}
