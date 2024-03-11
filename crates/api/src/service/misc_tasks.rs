use crate::Db;
use oxidized_config::get_config;
use oxidized_entity::{sea_orm::prelude::ConnectionTrait, sea_orm::DatabaseConnection};
use oxidized_service::Mutation;
use rocket::{
    fairing::{self, Fairing},
    Build, Rocket,
};
use sea_orm_rocket::Database;

pub struct MiscTasksService {}

#[rocket::async_trait]
impl Fairing for MiscTasksService {
    fn info(&self) -> fairing::Info {
        fairing::Info {
            name: "Miscellaneous Tasks Service",
            kind: fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let conn = &Db::fetch(&rocket).unwrap().conn;
        let config = get_config();

        self.spawn_vacuum(conn.clone());

        if config.app.clean {
            self.spawn_stale(conn.clone());
        }

        Ok(rocket)
    }
}
impl MiscTasksService {
    /// Create a new instance of the `MiscTasksService`
    ///
    /// ## Returns
    ///
    /// A new instance of the `MiscTasksService`
    pub fn new() -> Self {
        Self {}
    }

    /// Vacuum the database every hour
    ///
    /// ## Arguments
    ///
    /// * `conn` - A connection to the database
    pub fn spawn_vacuum(&self, conn: DatabaseConnection) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60 * 60));

        tokio::spawn(async move {
            loop {
                interval.tick().await;

                let vacuum = conn.execute_unprepared("VACUUM torrents").await;

                if let Err(e) = vacuum {
                    error!("Error vacuuming: {:?}", e);
                }
            }
        });
    }

    pub fn spawn_stale(&self, conn: DatabaseConnection) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60 * 60));

        tokio::spawn(async move {
            loop {
                interval.tick().await;

                Mutation::mark_stale(&conn).await.unwrap();
                // Wait for the next tracker scrape to do this for us
                // Mutation::delete_stale(&conn).await.unwrap();
            }
        });
    }
}
