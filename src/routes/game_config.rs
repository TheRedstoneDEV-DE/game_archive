use rocket::{routes, get, post, delete};
use rocket::http::Status;
use std::collections::HashMap;
use rocket_db_pools::Connection;
use rocket::serde::json;
use crate::structures::{Db, GameConfig, CompatTool};
use rocket_db_pools::sqlx;
use serde::{Serialize, Deserialize};


#[derive(Deserialize, Serialize)]
struct MetaCompatTool {
    pub id: i64,
    pub name: String
}

#[get("/launch_config?<id>")]
async fn get_game_config(id: i64, mut db:  Connection<Db>) -> Option<String>{
    Some(
        sqlx::query!(
            "SELECT launch_config FROM subgames WHERE id = ?",
            id
        ).fetch_optional(&mut **db)
        .await
        .ok()??
        .launch_config
    )
}

#[post("/launch_config?<id>", format="json", data="<data>")]
async fn post_game_config(mut db: Connection<Db>, data: json::Json<GameConfig>, id: i64) -> Option<json::Json<GameConfig>>{
    let stringified_json = json::serde_json::to_string_pretty(&data.clone().into_inner()).ok()?;
    sqlx::query!(
        "UPDATE subgames SET launch_config = ?1 WHERE id = ?",
        stringified_json,
        id
    ).execute(&mut **db)
    .await
    .ok()?;
    Some(data)
}

#[get("/compat_tools?<id>")]
async fn get_compat_tool(id: i64, mut db: Connection<Db>) -> Option<json::Json<CompatTool>> {
    let rows = sqlx::query!(
        "SELECT id, name, executable, environment FROM compat_tools WHERE id = ?",
        id
    ).fetch_optional(&mut **db)
    .await
    .ok()??;

    Some(
        json::Json(CompatTool { 
            id: rows.id,
            name: rows.name,
            executable: rows.executable,
            environment: json::serde_json::from_str::<HashMap<String, String>>(&rows.environment?).ok()? 
        })
    )
}

#[get("/compat_assign?<tool>&<game>")]
async fn get_compat_assign(tool: i64, game: i64, mut db: Connection<Db>) -> Option<Status> {
    sqlx::query!(
        "UPDATE subgames SET compat_tool = ?1 WHERE id = ?2",
        tool,
        game
    ).execute(&mut **db)
    .await
    .ok()?;
    
    Some(Status::Ok)
}

#[get("/compat_tools")]
async fn get_compat_tools(mut db: Connection<Db>) -> Option<json::Json<Vec<MetaCompatTool>>> {
    Some(json::Json(
        sqlx::query_as!(
             MetaCompatTool,
             "SELECT id, name FROM compat_tools"
         ).fetch_all(&mut **db)
         .await
         .ok()?
    ))
}

#[post("/compat_tools", format="json", data="<data>")]
async fn post_compat_tools(mut db: Connection<Db>, data: json::Json<CompatTool>) -> Option<Status> {
    let env_data = json::serde_json::to_string_pretty(&data.environment).ok()?;
    if data.id != 0 {
        sqlx::query!(
            "UPDATE compat_tools SET name = ?, executable = ?, environment = ? WHERE id = ?",
            data.name,
            data.executable,
            env_data,
            data.id
        ).execute(&mut **db)
        .await
        .ok()?;
        return Some(Status::Ok)
    }
    sqlx::query!(
        "INSERT INTO compat_tools (name,executable,environment) VALUES (?,?,?)",
        data.name,
        data.executable,
        env_data,
    ).execute(&mut **db)
    .await
    .ok()?;
    return Some(Status::Ok)
}

#[delete("/compat_tools?<id>")]
async fn delete_compat_tools(id: i64, mut db: Connection<Db>) -> Option<Status> {
    sqlx::query!(
        "DELETE FROM compat_tools WHERE id = ?",
        id
    ).execute(&mut **db)
    .await
    .ok()?;

    return Some(Status::Gone)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_game_config, post_game_config, get_compat_tool, post_compat_tools, get_compat_tools, get_compat_assign, delete_compat_tools]
}
