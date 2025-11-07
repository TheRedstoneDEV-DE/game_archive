use rocket::{routes, get, State, put};
use rocket::serde::json;
use std::sync::Arc;
use crate::structures::*;
use crate::database_helper;
use serde::{Serialize, Deserialize};
use rusqlite::params;


#[derive(Deserialize, Serialize)]
struct MetaCompatTool {
    pub id: u32,
    pub name: String
}

#[get("/game_conf?<id>")]
fn get_game_config(id: i64, db:  &State<Arc<DbConnection>>) -> Option<json::Json<GameConfig>>{
    let conn = db.0.lock().unwrap();
    return Some(json::Json(database_helper::get_game_conf(conn, id)?));
}

#[put("/game_conf?<id>", format="json", data="<data>")]
fn put_game_conf(db: &State<Arc<DbConnection>>, data: json::Json<GameConfig>, id: i64) -> Option<json::Json<GameConfig>>{
    let conn = db.0.lock().unwrap();
    let stringified_json = json::serde_json::to_string_pretty(&data.clone().into_inner()).ok()?;
    let _ = conn.execute("UPDATE games SET launch_conf = ?1 WHERE id = ?2)", params![stringified_json, id]).ok()?;
    Some(data)
}

#[get("/compat_tool?<id>")]
fn get_compat_tool(id: i64, db: &State<Arc<DbConnection>>) -> Option<json::Json<CompatTool>> {
    let conn = db.0.lock().unwrap();
    return Some(json::Json(database_helper::get_compat_tool(conn, id)?));
}

#[get("/compat_assign?<tool>&<game>")]
fn get_compat_assign(tool: i64, game: i64, db: &State<Arc<DbConnection>>) -> Option<String> {
    let ret_str: String = format!["{} -> {}", tool, game];
    let conn = db.0.lock().unwrap();
    let _ = conn.execute("UPDATE games SET compat_tool = ?1 WHERE id = ?2", params![tool, game]).ok()?;
    Some(ret_str)
}

#[get("/compat_tools")]
fn get_compat_tools(db: &State<Arc<DbConnection>>) -> Option<json::Json<Vec<MetaCompatTool>>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, name FROM compat_tools").ok()?;
    let compat_tools = stmt.query_map([], |row| {
        Ok(MetaCompatTool{
            id: row.get(0).unwrap(),
            name: row.get(1).unwrap()
        })
    }).ok()?;
    Some(json::Json(compat_tools.collect::<Result<Vec<_>, _>>().ok()?))
}

#[put("/compat_tool", format="json", data="<data>")]
fn put_compat_tool(db: &State<Arc<DbConnection>>, data: json::Json<CompatTool>) -> Option<json::Json<CompatTool>> {
    let conn = db.0.lock().unwrap();
    let env_data = json::serde_json::to_string(&data.environment).ok()?;
    if data.id != 0 {
        let _ = conn.execute("UPDATE compat_tools SET name = ?1, executable = ?2, environment = ?3 WHERE id = ?4", params![data.name, data.executable, env_data, data.id]).ok()?; 
        return Some(data);
    }
    let _ = conn.execute("INSERT INTO compat_tools(name, executable, environment) VALUES ?1, ?2, ?3", params![data.name, data.executable, env_data]).ok()?;
    Some(data)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_game_config, put_game_conf, get_compat_tool, get_compat_tools, put_compat_tool, get_compat_assign]
}
