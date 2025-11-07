use rocket::{get, put, Data, State, routes};
use rocket::http::ContentType;
use rocket::data::ToByteUnit;
use rocket::response::stream::ByteStream;
use rusqlite::params;
use std::io;
use std::sync::Arc;
use tokio::task;
use crate::structures::DbConnection;

fn write_buffer_blocking(data: Data<'_>, mut buffer: &mut Vec<u8>) -> io::Result<()> {
    use tokio::io::AsyncReadExt;
    task::block_in_place(|| {
        // now it's safe to block_on because we’re on a dedicated blocking thread
        rocket::tokio::runtime::Handle::current().block_on(async {
                data.open(5.mebibytes())
                .read_to_end(&mut buffer)
                .await
                .map(|_info| ())
        })
    }) 
}

#[get("/media?<game>&<location>")]
fn get_media(game: i32, location: String, db: &State<Arc<DbConnection>>) -> Option<(ContentType, ByteStream![Vec<u8>])> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT COALESCE(media_id, 0) as media_id FROM games WHERE id = ?1").ok()?;
    let rows = stmt.query_row(params![game], |row| {
        Ok(row.get::<usize, i64>(0))
    }).ok()?;
    if let Ok(media_id) = rows {
        let sql = format!("SELECT {}_blob, {}_mime_type FROM artworks WHERE id = ?1", location, location);
        let mut stmt_media = conn.prepare(&sql).ok()?;
        let (bytes, mime): (Vec<u8>, String) = stmt_media.query_row([media_id], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            let mime: String   = row.get(1)?;
            Ok((bytes, mime))
        }).ok()?;
        let ct = ContentType::parse_flexible(&mime).unwrap_or(ContentType::Binary);
       
        return Some((ct, ByteStream!{yield bytes;}));
    }
    None
}

#[put("/media?<game>&<location>", data="<data>")]
fn put_media(game: i32, location: String, db: &State<Arc<DbConnection>>, data: Data<'_>, content_type: &ContentType) -> String {
    if !(vec!["card".to_string(), "background".to_string()].contains(&location)) {
        return "{{\"status\": \"Wrong media location! It can only be card or background\"}}".to_string();
    }
    let conn = db.0.lock().unwrap();
    let mut find_media_statement = conn.prepare("SELECT COALESCE(media_id, 0) as media_id FROM games WHERE id = ?1").expect("Should be able to prepare Statement!");
    let media_id_query = find_media_statement.query_map(params![game], |row|{
        Ok(row.get::<usize, u64>(0))
    }).expect("Statement should not fail!").collect::<Result<Vec<_>, _>>().expect("Should be able to collect.");
    if let Ok(media_id) = &media_id_query[0] {
        let data_mimetype = content_type.to_string();
        let mut buffer: Vec<u8> = Vec::new();
        write_buffer_blocking(data, &mut buffer).unwrap();

        if *media_id == 0 {
            let sql = format!("INSERT INTO artworks({}_mime_type, {}_blob) VALUES (?1, ?2)", location, location);
            let rows_updated_1 = conn.execute(&sql,
                params![data_mimetype, buffer]).unwrap_or_else(|_| {
                0
            });
            if rows_updated_1 == 0 {
                return "{{\"status\": \"Could not update artworks Database!\"}}".to_string();
            }

            let rows_updated = conn.execute("UPDATE games SET media_id = ?1 WHERE id = ?2",
                params![conn.last_insert_rowid(), game]).unwrap_or_else(|_| {
                0
            });
            if rows_updated == 0 {
                return "{{\"status\": \"Could not update games Database!\"}}".to_string();
            }
        } else {
            let sql = format!("UPDATE artworks SET {}_mime_type = ?1, {}_blob = ?2 WHERE id = ?3", location, location);
            let rows_updated_1 = conn.execute(&sql,
                params![data_mimetype, buffer, media_id]).unwrap_or_else(|_| {
                0
            });
            if rows_updated_1 == 0 {
                return "{{\"status\": \"Could not update artworks Database!\"}}".to_string();
            }
        }

    }
    "{{\"status\": \"Success!\"}}".to_string()
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_media, put_media]
}
