use base64::{Engine, prelude::BASE64_STANDARD};
use png::{Decoder, Encoder};
use rocket::data::ToByteUnit;
use tokio::io::AsyncReadExt;
use std::io::Cursor;
use image::ImageReader;
use rocket::{get, post, response::stream::ByteStream, Data, routes};
use rocket::http::{ContentType, Status};
use rocket::serde::json;
use rocket_db_pools::{Connection, sqlx};

use crate::structures::{DataExport, Db, Game, GameConfig, GameExport, MediaObj, SubGame};

async fn get_gameobj(id: i64, db: &mut Connection<Db>) -> Option<Game> {
    let subgames = sqlx::query_as!(
        SubGame,
        "SELECT id, name, playtime, last_launch, is_archived, parent FROM subgames WHERE parent = ?",
        id
    ).fetch_all(&mut ***db)
    .await
    .ok();
    let name = sqlx::query!(
        "SELECT name FROM games WHERE id = ?",
        id
    ).fetch_optional(&mut ***db)
    .await
    .ok()??.name;
    
    Some(Game{
        id: id.into(),
        name,
        subgames
    })
}

async fn get_media(id: i64, location: Option<&str>, db: &mut Connection<Db>) -> Option<MediaObj> {
    if let Some(location) = location {
        let row = sqlx::query!(
            "SELECT blob, mime_type FROM artworks WHERE type = ? AND game = ?",
            location,
            id
        ).fetch_optional(&mut ***db)
        .await
        .ok()??;
        return Some(MediaObj{
            base64: BASE64_STANDARD.encode(row.blob?),
            format: row.mime_type?,
        });
    }
    let row = sqlx::query!(
        "SELECT image, mime_type FROM subgame_covers WHERE subgame = ?",
        id
    ).fetch_optional(&mut ***db)
    .await
    .ok()??;
    Some(MediaObj{
        base64: BASE64_STANDARD.encode(row.image),
        format: row.mime_type,
    })
}

async fn get_launch_config(id: i64, db: &mut Connection<Db>) -> Option<GameConfig> {
    let config = sqlx::query!(
            "SELECT launch_config FROM subgames WHERE id = ?",
            id
        ).fetch_optional(&mut ***db)
        .await
        .ok()??
        .launch_config;
    Some(json::from_str(&config).ok()?)
}

async fn pack(data: DataExport) -> Option<Vec<u8>> {
    let container_img = ImageReader::new(Cursor::new(BASE64_STANDARD.decode(data.game_card.clone()?.base64).ok()?)).with_guessed_format().ok()?.decode().ok()?.to_rgb8();
    let (width, height) = container_img.dimensions();

    let data_json = json::to_string(&data).ok()?;
    let mut output: Vec<u8> = Vec::new();

    let mut encoder = Encoder::new(&mut output, width, height);
    {
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.add_ztxt_chunk("configuration".to_string(), data_json).ok()?;

        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(container_img.as_raw()).ok()?;
    }

    Some(output)
}

#[get("/xport?<id>")]
async fn get_xport(id: i64, mut db: Connection<Db>) -> Option<(ContentType, ByteStream![Vec<u8>])> { 
    let game = get_gameobj(id, &mut db).await?;
    let mut subgames: Vec<GameExport> = Vec::new();

    for subgame in game.subgames? {
        println!("Adding {} (id={})...",&subgame.name, &subgame.id);
        subgames.push(GameExport {
            name: subgame.name,
            playtime: subgame.playtime,
            last_launch: subgame.last_launch,
            is_archived: subgame.is_archived,
            config: get_launch_config(subgame.id, &mut db).await?,
            game_card: get_media(subgame.id, None, &mut db).await,
        });
    }

    let data = DataExport {
        game_name: game.name,
        subgames,
        game_card: get_media(id, Some("card"), &mut db).await,
        game_background: get_media(id, Some("background"), &mut db).await,
    };

    let packed_image = pack(data).await?;


    Some((ContentType::PNG, ByteStream!{ yield packed_image; }))
}

async fn add_media(id: i32, location: Option<&str>, media: MediaObj, db: &mut Connection<Db>) -> Option<()> {
    let content_type = media.format;
    let blob = BASE64_STANDARD.decode(media.base64).ok()?;
    if let Some(location) = location {
        sqlx::query!(
            r#"INSERT INTO artworks (mime_type, blob, type, game) VALUES (?,?,?,?) 
                ON CONFLICT (game, type) DO UPDATE SET 
                blob = excluded.blob,
                mime_type = excluded.mime_type"#,
            content_type,
            blob,
            location,
            id
        ).execute(&mut ***db)
        .await
        .ok()?;
        return Some(());
    }

    sqlx::query!(
        r#"INSERT INTO subgame_covers (mime_type, image, subgame) VALUES (?,?,?) 
            ON CONFLICT (subgame) DO UPDATE SET 
            image = excluded.image,
            mime_type = excluded.mime_type"#,
        content_type,
        blob,
        id
    ).execute(&mut ***db)
    .await
    .ok()?;
    return Some(());
}

async fn add_game(name: String, subgame: Option<SubGame>, db: &mut Connection<Db>) -> Option<i32> {
    if let Some(subgame) = subgame {
       let row = sqlx::query!(
            "INSERT INTO subgames (name,playtime,last_launch,is_archived,parent) VALUES (?,?,?,?,?); SELECT last_insert_rowid() AS id;",
            subgame.name,
            subgame.playtime,
            subgame.last_launch,
            subgame.is_archived,
            subgame.parent
        ).fetch_optional(&mut ***db)
        .await
        .ok()??;
        
        return Some(row.id);
    }

    let row = sqlx::query!(
        "INSERT INTO games (name) VALUES (?); SELECT last_insert_rowid() AS id",
        name
    ).fetch_optional(&mut ***db)
    .await
    .ok()??;
    
    Some(row.id)
}

async fn get_first_compat_tool(db: &mut Connection<Db>) -> Option <i64> {
    let row = sqlx::query!(
        "SELECT id FROM compat_tools LIMIT 1"
    ).fetch_optional(&mut ***db)
    .await
    .ok()??;
    Some(row.id)
}

async fn unpack(bin_data: Vec<u8>) -> Option<DataExport> {
    let decoder = Decoder::new(Cursor::new(bin_data));
    let reader = decoder.read_info().ok()?;
    let info = reader.info();

    let chunk = info.compressed_latin1_text.iter().find(|c| c.keyword == "configuration")?;
    
    let json_data = chunk.get_text().ok()?;
    Some(json::from_str(&json_data).ok()?)
}

#[post("/xport", data="<data>")]
async fn post_xport(data: Data<'_>, content_type: &ContentType, mut db: Connection<Db>) -> Option<Status> {
    if *content_type != ContentType::PNG {
        return Some(Status::ImATeapot);
    }

    if let Some(first_compat_id) = get_first_compat_tool(&mut db).await { 
        let mut buffer: Vec<u8> = Vec::new();
        data.open(10.mebibytes()).read_to_end(&mut buffer).await.map(|_info|()).ok()?;
        let export: DataExport = unpack(buffer).await?;
        let game_id = add_game(export.game_name, None, &mut db).await?;
        
        add_media(game_id, Some("card"), export.game_card?, &mut db).await?;
        if let Some(game_background) = export.game_background {
            add_media(game_id, Some("background"), game_background, &mut db).await?;
        }

        for subgame in export.subgames {
            let sub_id = add_game("".to_string(), Some(SubGame{
                id: 0,
                name: subgame.name,
                playtime: subgame.playtime,
                last_launch: subgame.last_launch,
                is_archived: subgame.is_archived,
                parent: game_id.into(),
            }), &mut db).await?;

            if let Some(artwork) = subgame.game_card {
                add_media(sub_id, None, artwork, &mut db).await?;
            }

            let config = json::to_pretty_string(&subgame.config).ok()?;
            sqlx::query!(
                "UPDATE subgames SET launch_config = ?, compat_tool = ? WHERE id = ?",
                config,
                first_compat_id,
                sub_id
            ).execute(&mut **db)
            .await
            .ok()?;
        }
        return Some(Status::Ok);
    }
    Some(Status::BadRequest)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_xport, post_xport]
}
