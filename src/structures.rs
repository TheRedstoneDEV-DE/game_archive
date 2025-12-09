use std::sync::{atomic::AtomicBool, atomic::AtomicIsize, Mutex, atomic::AtomicU32};
use std::collections::HashMap;
use rocket::serde::{Deserialize, Serialize};
use rocket_db_pools::{sqlx, Database};

#[derive(Database)]
#[database("sqlite_db")]
pub struct Db(sqlx::SqlitePool);

#[derive(PartialEq, Eq, Deserialize, Serialize, Clone)]
pub enum HistoryType {
    MONTH,
    WEEK,
    DAY
}

#[derive(Deserialize, Serialize)]
pub struct HistoryGame {
    pub id: i64,
    pub playtime: i64
}

#[derive(Deserialize, Serialize)]
pub struct GameHistory {
    pub r#type: HistoryType,
    pub date: String,
    pub games: Vec<HistoryGame>
}

#[derive(Deserialize, Serialize)]
pub struct MetaGame {
    pub id: u32,
    pub name: String,
    pub playtime: f32,
    pub last_launch: i64,
    pub archived: bool,
}

#[derive(Deserialize, Serialize)]
pub struct Game {
    pub id: u32,
    pub name: String,
    pub sub_games: Vec<SubGame>,
}

#[derive(Deserialize, Serialize)]
pub struct SubGame {
    pub id: u32,
    pub name: String,
    pub playtime: f32,
    pub last_launch: i64,
    pub archived: bool,
}

#[derive(Deserialize, Serialize)]
pub struct CompatTool {
   pub id: u32,
   pub name:String,
   pub executable: String,
   pub environment: HashMap<String,String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct GameConfig {
    pub arguments: Vec<String>,
    pub working_directory: String,
    pub game_prefix: String,
    pub executable: String,
    pub environment: HashMap<String,String>,
    pub archive_file: String,
}

pub struct GameRuntime {
    pub current_game: AtomicIsize, 
    pub game_running: AtomicBool,
    pub running_since: AtomicIsize,
    pub pid: AtomicU32, 
}

