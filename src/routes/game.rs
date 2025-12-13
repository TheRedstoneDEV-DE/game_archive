use rocket::{get, post, routes, delete};
use rocket::http::Status;
use rocket::serde::json;
use rocket_db_pools::sqlx;
use rocket_db_pools::Connection;
use crate::structures::{Db, MetaGame, SubGame, Game};

#[post("/subgame", format = "json", data = "<data>")]
async fn post_subgame(mut db: Connection<Db>, data: json::Json<SubGame>) -> Option<Status> {
    if data.id == 0 {
        sqlx::query!(
            "INSERT INTO subgames (name,playtime,last_launch,is_archived,parent) VALUES (?,?,?,?,?)",
            data.name,
            data.playtime,
            data.last_launch,
            data.is_archived,
            data.parent
        ).execute(&mut **db)
        .await
        .ok()?;

        return Some(Status::Ok)
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

    Some(Status::Ok)
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
            JOIN subgames s ON g.id = s.parent 
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
                  MIN(s.is_archived) AS is_archived
            FROM games g 
            JOIN subgames s ON g.id = s.parent
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
async fn post_games(mut db: Connection<Db>, data: json::Json<Game>) -> Option<Status> {
    if data.id == 0 {
        sqlx::query!(
            "INSERT INTO games (name) VALUES (?)",
            data.name
        ).execute(&mut **db)
        .await
        .ok()?;

        return Some(Status::Ok)
    }
    sqlx::query!(
        "UPDATE games SET name = ? WHERE id = ?",
        data.name,
        data.id
    ).execute(&mut **db)
    .await
    .ok()?;

    Some(Status::Ok)
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
