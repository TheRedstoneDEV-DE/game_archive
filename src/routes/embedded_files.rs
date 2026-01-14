use rocket::{get, routes, Route};
use rocket::http::ContentType;
use rust_embed::RustEmbed;
use rocket::response::content::RawHtml;


use std::borrow::Cow;
use std::ffi::OsStr;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

#[get("/<path..>", rank = 12)]
fn static_files(path: std::path::PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = path.display().to_string();
    let asset = Assets::get(&filename)?;
    let content_type = path
      .extension()
      .and_then(OsStr::to_str)
      .and_then(ContentType::from_extension)
      .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

#[get("/", rank = 11)]
fn index() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let asset = Assets::get("index.html")?;
    Some(RawHtml(asset.data))
}

pub fn routes() -> Vec<Route> {
    routes![static_files, index]
}

