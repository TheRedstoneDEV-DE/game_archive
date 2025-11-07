use std::sync::MutexGuard;
use std::collections::HashMap;
use rusqlite::{Connection, params};
use rocket::serde::json::serde_json;

use crate::structures::{CompatTool, GameConfig};

pub fn create_tables(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute(
         "CREATE TABLE IF NOT EXISTS artworks (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             card_mime_type TEXT,
             card_blob BLOB,
             background_mime_type TEXT,
             background_blob BLOB
         )",
         [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS games (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            is_subgame BOOL NOT NULL,
            related_to INTEGER,
            playtime REAL,
            last_launch INTEGER,
            is_archived BOOL,
            FOREIGN KEY('media_id') REFERENCES 'artworks'('id'),
            FOREIGN KEY('compat_tool') REFERENCES 'compat_tools'('id')
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS compat_tools (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            executable TEXT NOT NULL,
            environment TEXT
        )",
        []
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_start INTEGER NOT NULL,
            timestamp_end INTEGER NOT NULL,
            game INTEGER NOT NULL,
            FOREIGN KEY('game') REFERENCES 'games'('id')
        )",
        []
    )?; 
    Ok(())
}

pub fn get_compat_tool(conn: MutexGuard<'_,Connection>, id: i64) -> Option<CompatTool> {
    let mut stmt = conn.prepare("SELECT compat_tools.* FROM games JOIN compat_tools ON games.compat_tool = compat_tools.id WHERE games.id = ?1").ok()?;
    let (comp_id, name, executable, environment): (Result<i64,_>, Result<String,_>, Result<String,_>, Result<String,_>) = stmt.query_row(params![id], |row| {
        let id = row.get::<usize, i64>(0);
        let name = row.get::<usize, String>(1);
        let executable = row.get::<usize, String>(2);
        let environment = row.get::<usize, String>(3);
        Ok((id, name, executable, environment))
    }).ok()?;
    Some(
        CompatTool {
            id: comp_id.ok()? as u32,
            name: name.ok()?,
            executable: executable.ok()?,
            environment: serde_json::from_str::<HashMap<String, String>>(&environment.ok()?).ok()?,
        }
    )
}

pub fn get_game_conf(conn: MutexGuard<'_,Connection>, id: i64) -> Option<GameConfig> {
    let mut stmt = conn.prepare("SELECT games.launch_conf FROM games WHERE games.id = ?1").ok()?;
    let game_config_string: String = stmt.query_row(params![id], |row| {
        Ok(row.get::<usize, String>(0))
    }).ok()?.ok()?;
    Some(serde_json::from_str::<GameConfig>(&game_config_string).ok()?)
}

pub fn add_to_history(conn: MutexGuard<'_, Connection>, timestamp_start: i64, timestamp_end: i64, game: i64) -> Result<(), Box<dyn std::error::Error>>{
    let mut stmt = conn.prepare("SELECT timestamp_end, id FROM history WHERE game = ?1 ORDER BY timestamp_end DESC LIMIT 1")?;
    let row: Result<(Result<i64, _>, Result<i64, _>), _> = stmt.query_row(params![game], |row| {
        Ok((row.get::<usize, i64>(0), row.get::<usize, i64>(1)))
    });
    if let Ok((Ok(last_end_timestamp), Ok(row_id))) = row{
        if timestamp_start - 600 <= last_end_timestamp {
            conn.execute(
                "UPDATE history SET timestamp_end = ?1 WHERE id = ?2",
                params![timestamp_end, row_id]
            )?;
            return Ok(());
        } 
    }
    conn.execute(
        "INSERT INTO history(timestamp_start, timestamp_end, game) VALUES(?1, ?2, ?3)",
        params![timestamp_start, timestamp_end, game]
    )?;
    Ok(())
}
