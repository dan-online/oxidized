use std::io::Cursor;

use oxidized_entity::torrent::Model as Torrent;
use oxidized_service::Query;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use rocket::http::uri::Host;
use rocket::http::{ContentType, Status};
use sea_orm_rocket::Connection;

use crate::guards::apikey::ApiKeyGuard;
use crate::Db;

#[derive(FromForm, Debug)]
pub struct TorznabQuery<'a> {
    t: Option<&'a str>,
    q: Option<String>,
    offset: Option<u64>,
    limit: Option<u64>,
}

// let categories_to_add = vec![("8000", "Other"), ("2000", "Movies"), ("5000", "TV")];
static CATEGORIES_TO_ADD: &[(&str, &str, &[(&str, &str)])] = &[
    ("8000", "Other", &[("8010", "Other/Misc")]),
    ("2000", "Movies", &[]),
    ("5000", "TV", &[("5040", "TV/HD"), ("5070", "TV/SD")]),
];

fn generate_caps_response() -> String {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();

    let caps = BytesStart::new("caps");

    writer.write_event(Event::Start(caps)).unwrap();

    let mut server = BytesStart::new("server");
    server.push_attribute(("version", "1.0"));
    server.push_attribute(("title", "Oxidized"));

    writer.write_event(Event::Empty(server)).unwrap();

    let mut limits = BytesStart::new("limits");
    limits.push_attribute(("max", "100"));
    limits.push_attribute(("default", "50"));

    writer.write_event(Event::Empty(limits)).unwrap();

    let searching = BytesStart::new("searching");
    writer.write_event(Event::Start(searching)).unwrap();

    let mut search = BytesStart::new("search");
    search.push_attribute(("available", "yes"));
    search.push_attribute(("supportedParams", "q"));

    writer.write_event(Event::Empty(search)).unwrap();

    writer
        .write_event(Event::End(BytesEnd::new("searching")))
        .unwrap();

    let categories = BytesStart::new("categories");

    writer.write_event(Event::Start(categories)).unwrap();

    for (id, name, subcats) in CATEGORIES_TO_ADD {
        let mut category = BytesStart::new("category");

        category.push_attribute(("id", *id));
        category.push_attribute(("name", *name));
        category.push_attribute(("description", *name));

        // writer.write_event(Event::Empty(category)).unwrap();
        if !subcats.is_empty() {
            writer.write_event(Event::Start(category)).unwrap();

            for (sub_id, sub_name) in *subcats {
                let mut subcat = BytesStart::new("subcat");

                subcat.push_attribute(("id", *sub_id));
                subcat.push_attribute(("name", *sub_name));

                writer.write_event(Event::Empty(subcat)).unwrap();
            }

            writer
                .write_event(Event::End(BytesEnd::new("category")))
                .unwrap();
        } else {
            writer.write_event(Event::Empty(category)).unwrap();
        }
    }

    writer
        .write_event(Event::End(BytesEnd::new("categories")))
        .unwrap();

    writer
        .write_event(Event::End(BytesEnd::new("caps")))
        .unwrap();

    String::from_utf8(writer.into_inner().into_inner()).unwrap()
}

fn generate_search_response(origin: &Host, torrents: Vec<Torrent>) -> anyhow::Result<String> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))?;

    let rss_name = "rss";
    let mut element = BytesStart::new(rss_name);
    element.push_attribute(("version", "2.0"));
    element.push_attribute(("xmlns:atom", "http://www.w3.org/2005/Atom"));
    element.push_attribute(("xmlns:torznab", "http://torznab.com/schemas/2015/feed"));

    writer.write_event(Event::Start(element))?;

    let channel = BytesStart::new("channel");

    writer.write_event(Event::Start(channel)).unwrap();

    writer
        .create_element("title")
        .write_text_content(BytesText::new("Latest releases feed"))?;
    writer
        .create_element("link")
        .write_text_content(BytesText::new(format!("http://{}/", origin).as_str()))?;
    writer
        .create_element("description")
        .write_text_content(BytesText::new("Latest releases feed"))?;
    writer
        .create_element("language")
        .write_text_content(BytesText::new("en-gb"))?;
    writer
        .create_element("ttl")
        .write_text_content(BytesText::new("30"))?;

    // Iterate through torrents to add items
    for torrent in torrents {
        let item = BytesStart::new("item");

        writer.write_event(Event::Start(item))?;

        writer
            .create_element("title")
            .write_text_content(BytesText::new(torrent.name.unwrap().as_str()))?;
        writer
            .create_element("link")
            .write_text_content(BytesText::new(
                format!(
                    "https://itorrents.org/torrent/{}.torrent",
                    torrent.info_hash
                )
                .as_str(),
            ))?;
        writer
            .create_element("description")
            .write_text_content(BytesText::new(
                format!("Total Size: {} MB", torrent.size).as_str(),
            ))?;
        writer
            .create_element("pubDate")
            .write_text_content(BytesText::new(torrent.added_at.to_string().as_str()))?;
        writer
            .create_element("size")
            .write_text_content(BytesText::new(
                (torrent.size as i64 * 1000000).to_string().as_str(),
            ))?;
        writer
            .create_element("infohash")
            .write_text_content(BytesText::new(torrent.info_hash.as_str()))?;
        writer
            .create_element("category")
            .write_text_content(BytesText::new("Other"))?;
        writer
            .create_element("seeders")
            .write_text_content(BytesText::new(torrent.seeders.to_string().as_str()))?;
        writer
            .create_element("leechers")
            .write_text_content(BytesText::new(
                (torrent.seeders + torrent.leechers).to_string().as_str(),
            ))?;
        writer
            .create_element("magneturl")
            .write_text_content(BytesText::new(
                format!("magnet:?xt=urn:btih:{}", torrent.info_hash.to_uppercase()).as_str(),
            ))?;

        let mut enc = BytesStart::new("enclosure");
        enc.push_attribute(("type", "application/x-bittorrent"));
        enc.push_attribute((
            "url",
            format!(
                "https://itorrents.org/torrent/{}.torrent",
                torrent.info_hash
            )
            .as_str(),
        ));

        writer.write_event(Event::Empty(enc)).unwrap();

        writer
            .create_element("torznab:attr")
            .with_attribute(("name", "files"))
            .with_attribute(("value", torrent.files.len().to_string().as_str()))
            .write_empty()?;

        writer
            .create_element("torznab:attr")
            .with_attribute(("name", "size"))
            .with_attribute((
                "value",
                (torrent.size as i64 * 1000000).to_string().as_str(),
            ))
            .write_empty()?;

        writer
            .create_element("torznab:attr")
            .with_attribute(("name", "infohash"))
            .with_attribute(("value", torrent.info_hash.as_str()))
            .write_empty()?;

        writer
            .create_element("torznab:attr")
            .with_attribute(("name", "magneturl"))
            .with_attribute((
                "value",
                format!("magnet:?xt=urn:btih:{}", torrent.info_hash.to_uppercase()).as_str(),
            ))
            .write_empty()?;

        writer
            .create_element("torznab:attr")
            .with_attribute(("name", "seeders"))
            .with_attribute(("value", torrent.seeders.to_string().as_str()))
            .write_empty()?;

        writer
            .create_element("torznab:attr")
            .with_attribute(("name", "peers"))
            .with_attribute((
                "value",
                (torrent.seeders + torrent.leechers).to_string().as_str(),
            ))
            .write_empty()?;

        // for (id, _, _) in CATEGORIES_TO_ADD {
        // writer
        //     .create_element("torznab:attr")
        //     .with_attribute(("name", "category"))
        //     .with_attribute(("value", "8010"))
        //     .write_empty()?;
        // }
        for (id, _, subcats) in CATEGORIES_TO_ADD {
            if !subcats.is_empty() {
                for (subcat_id, _) in subcats.iter() {
                    writer
                        .create_element("torznab:attr")
                        .with_attribute(("name", "category"))
                        .with_attribute(("value", *subcat_id))
                        .write_empty()?;
                }
            } else {
                writer
                    .create_element("torznab:attr")
                    .with_attribute(("name", "category"))
                    .with_attribute(("value", *id))
                    .write_empty()?;
            }
        }

        writer.write_event(Event::End(BytesEnd::new("item")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("channel")))?;

    writer.write_event(Event::End(BytesEnd::new(rss_name)))?;

    let xml_string = String::from_utf8(writer.into_inner().into_inner()).unwrap();

    Ok(xml_string)
}

#[get("/api?<query..>")]
pub async fn route<'a>(
    _apikey: ApiKeyGuard,
    conn: Connection<'_, Db>,
    query: TorznabQuery<'_>,
    origin: &Host<'_>,
) -> (Status, (ContentType, String)) {
    let conn = conn.into_inner();

    match query.t.unwrap_or("search") {
        "caps" => (Status::Ok, (ContentType::XML, generate_caps_response())),
        "search" => {
            let torrents =
                Query::search_torrents_by_name(&conn, query.q, query.offset, query.limit)
                    .await
                    .expect("Cannot search torrents");

            (
                Status::Ok,
                (
                    ContentType::XML,
                    generate_search_response(origin, torrents).unwrap(),
                ),
            )
        }
        _ => (
            Status::NotFound,
            (ContentType::Text, "Not Found".to_string()),
        ),
    }
}
