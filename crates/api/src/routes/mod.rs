use rocket::Route;

pub mod add;
pub mod get;
pub mod index;
pub mod list;
pub mod stats;
pub mod torznab;

pub fn get_routes() -> Vec<Route> {
    routes![
        list::route,
        add::route,
        get::route,
        index::route,
        stats::route,
        torznab::route,
    ]
}
