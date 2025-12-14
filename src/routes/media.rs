use rocket::{get, post, Data, routes};
use rocket::http::{ContentType,Status};
use rocket::data::ToByteUnit;
use rocket::response::stream::ByteStream;
use tokio::io::AsyncReadExt;
use crate::structures::Db;
use rocket_db_pools::{Connection, sqlx};

#[get("/media?<game>&<location>")]
async fn get_media(game: i32, location: String, mut db: Connection<Db>) -> Option<(ContentType, ByteStream![Vec<u8>])> {
    let row = sqlx::query!(
        "SELECT blob, mime_type FROM artworks WHERE type = ? AND game = ?",
        location,
        game
    ).fetch_optional(&mut **db)
    .await
    .ok()??;
    let ct = ContentType::parse_flexible(&row.mime_type?).unwrap_or(ContentType::Binary);
    let blob = row.blob?; 
    Some((ct, ByteStream!{yield blob;}))
}

#[post("/media?<game>&<location>", data="<data>")]
async fn post_media(game: i64, location: String, mut db: Connection<Db>, data: Data<'_>, content_type: &ContentType) -> Option<Status> {
    let content_type = content_type.to_string();
    let mut buffer: Vec<u8> = Vec::new();
    data.open(5.mebibytes()).read_to_end(&mut buffer).await.map(|_info|()).ok()?;
    sqlx::query!(
        r#"INSERT INTO artworks (mime_type, blob, type, game) VALUES (?,?,?,?) 
            ON CONFLICT (game, type) DO UPDATE SET 
            blob = excluded.blob,
            mime_type = excluded.mime_type"#,
        content_type,
        buffer,
        location,
        game
    ).execute(&mut **db)
    .await
    .ok()?;

    Some(Status::Ok)
}

#[get("/subgame_cover?<id>")]
async fn get_media_subgame(id: i32, mut db: Connection<Db>) -> Option<(ContentType, ByteStream![Vec<u8>])> {
    let row = sqlx::query!(
        "SELECT image, mime_type FROM subgame_covers WHERE subgame = ?",
        id
    ).fetch_optional(&mut **db)
    .await
    .ok()??;
    let ct = ContentType::parse_flexible(&row.mime_type).unwrap_or(ContentType::Binary);
    let blob = row.image; 
    Some((ct, ByteStream!{yield blob;}))
}

#[post("/subgame_cover?<id>", data="<data>")]
async fn post_media_subgame(id: i64, mut db: Connection<Db>, data: Data<'_>, content_type: &ContentType) -> Option<Status> {
    let content_type = content_type.to_string();
    let mut buffer: Vec<u8> = Vec::new();
    data.open(5.mebibytes()).read_to_end(&mut buffer).await.map(|_info|()).ok()?;
    sqlx::query!(
        r#"INSERT INTO subgame_covers (mime_type, image, subgame) VALUES (?,?,?) 
            ON CONFLICT (subgame) DO UPDATE SET 
            image = excluded.image,
            mime_type = excluded.mime_type"#,
        content_type,
        buffer,
        id
    ).execute(&mut **db)
    .await
    .ok()?;

    Some(Status::Ok)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_media, post_media, get_media_subgame, post_media_subgame]
}
