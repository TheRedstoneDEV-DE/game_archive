use rocket::{get, post, routes, delete};
use rocket::http::Status;
use rocket::serde::json;
use rocket_db_pools::sqlx;
use rocket_db_pools::Connection;
use crate::structures::{Db, MetaGame, SubGame, Game};

#[post("/subgame", format = "json", data = "<data>")]
async fn post_subgame(mut db: Connection<Db>, data: json::Json<SubGame>) -> Option<json::Json<SubGame>> {
    if data.id == 0 {
        let row = sqlx::query!(
            "INSERT INTO subgames (name,playtime,last_launch,is_archived,parent) VALUES (?,?,?,?,?); SELECT last_insert_rowid() AS id;",
            data.name,
            data.playtime,
            data.last_launch,
            data.is_archived,
            data.parent
        ).fetch_optional(&mut **db)
        .await
        .ok()??;

        return Some(json::Json(SubGame{
            id: row.id as i64,
            name: data.name.clone(),
            playtime: data.playtime,
            last_launch: data.last_launch,
            is_archived: data.is_archived,
            parent: data.parent
        }));
    }
    sqlx::query!(
        "UPDATE subgames SET name = ?, playtime = ?, last_launch = ?, is_archived = ?, parent = ? WHERE id = ?",
        data.name,
        data.playtime,
        data.last_launch,
        data.is_archived,
        data.parent,
        data.id
    ).execute(&mut **db)
    .await
    .ok()?;

    Some(data)
}

#[get("/subgame?<id>")]
async fn get_subgame(id: i64, mut db: Connection<Db>) -> Option<json::Json<SubGame>> {
    let subgame = sqlx::query_as!(
        SubGame,
        "SELECT id, name, playtime, last_launch, is_archived, parent FROM subgames WHERE id = ?",
        id
    ).fetch_optional(&mut **db)
    .await
    .ok()??;

    Some(json::Json(subgame))
}

#[get("/gamemeta?<id>")]
async fn get_gamemeta(id: i64, mut db: Connection<Db>) -> Option<json::Json<MetaGame>> {
    let game_meta = sqlx::query_as!(
        MetaGame,
        r#"SELECT g.id AS id,
                  g.name AS name, 
                  SUM(s.playtime) AS playtime, 
                  MAX(s.last_launch) AS last_launch, 
                  MIN(s.is_archived) AS is_archived 
            FROM games g 
            LEFT JOIN subgames s ON g.id = s.parent 
            WHERE g.id = ?"#,
        id
    ).fetch_optional(&mut **db)
    .await
    .ok()??;

    Some(json::Json(game_meta))
}

#[get("/games")]
async fn get_games(mut db: Connection<Db>) -> Option<json::Json<Vec<MetaGame>>> {
    let rows = sqlx::query!(
        r#"SELECT g.id AS id,
                  g.name AS name, 
                  SUM(s.playtime) AS playtime, 
                  MAX(s.last_launch) AS last_launch,
                  COALESCE(MIN(s.is_archived), 0) AS is_archived
            FROM games g 
            LEFT JOIN subgames s ON g.id = s.parent
            GROUP BY g.id, g.name
        "#
    ).fetch_all(&mut **db)
    .await
    .ok()?;

    let games: Vec<MetaGame> = rows.into_iter().map(|row| {
        MetaGame {
            id: row.id,
            name: row.name,
            playtime: row.playtime,
            last_launch: row.last_launch,
            is_archived: Some(row.is_archived != 0), // Manually convert i32 to bool, because SQLx does dumb suff
        }
    }).collect();


    Some(json::Json(games))
}

#[post("/games", format="json", data="<data>")]
async fn post_games(mut db: Connection<Db>, data: json::Json<Game>) -> Option<json::Json<Game>> {
    if data.id == 0 {
        let row = sqlx::query!(
            "INSERT INTO games (name) VALUES (?); SELECT last_insert_rowid() AS id",
            data.name
        ).fetch_optional(&mut **db)
        .await
        .ok()??;

        return Some(json::Json(Game{
            id: row.id as i64,
            name: data.name.clone(),
            subgames: vec![].into()
        }))
    } else {

    }
    sqlx::query!(
        "UPDATE games SET name = ? WHERE id = ?",
        data.name,
        data.id
    ).execute(&mut **db)
    .await
    .ok()?;

    Some(data)
}


#[get("/games?<id>")]
async fn get_game(id: i64, mut db: Connection<Db>) -> Option<json::Json<Game>> {
    let subgames = sqlx::query_as!(
        SubGame,
        "SELECT id, name, playtime, last_launch, is_archived, parent FROM subgames WHERE parent = ?",
        id
    ).fetch_all(&mut **db)
    .await
    .ok();
    let name = sqlx::query!(
        "SELECT name FROM games WHERE id = ?",
        id
    ).fetch_optional(&mut **db)
    .await
    .ok()??.name;

    Some(json::Json(Game{
        id,
        name,
        subgames
    }))
}

#[delete("/subgames?<id>")]
async fn delete_subgame(id: i64, mut db: Connection<Db>) -> Option<Status> {
    sqlx::query!(
        "DELETE FROM subgames WHERE id = ?",
        id
    ).execute(&mut **db)
    .await
    .ok()?;

    return Some(Status::Gone)
}

#[delete("/games?<id>")]
async fn delete_game(id: i64, mut db: Connection<Db>) -> Option<Status> {
    sqlx::query!(
        "DELETE FROM games WHERE id = ?",
        id
    ).execute(&mut **db)
    .await
    .ok()?;

    return Some(Status::Gone)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![post_subgame, post_games, get_gamemeta, get_game, get_games, get_subgame, delete_subgame, delete_game]
}
