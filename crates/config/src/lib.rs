use config::{Config, Environment, File};
use serde::Serialize;
use serde_derive::Deserialize;

#[derive(Deserialize, Serialize, Clone)]
pub struct AuthSettings {
    pub apikey: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct DatabaseSettings {
    pub url: String,
    pub sqlx_logging: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct AppSettings {
    pub spider: bool,
    pub update_info: bool,
    pub update_trackers: bool,
    pub clean: bool,
    pub filter_nsfw: bool,
}

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub app: AppSettings,
    pub auth: AuthSettings,
}

pub fn get_config() -> Settings {
    let settings = Config::builder()
        .add_source(File::with_name("default"))
        .add_source(File::with_name("config").required(false))
        .add_source(
            Environment::with_prefix("OXIDIZED")
                .separator("_")
                .try_parsing(true),
        )
        .build()
        .unwrap();

    settings.try_deserialize().unwrap()
}
