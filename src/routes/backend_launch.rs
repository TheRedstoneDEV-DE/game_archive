use rocket::{get, routes, State};
use rocket::serde::json;
use chrono::Utc;
use uuid::timestamp;
use std::thread;
use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc};
use tokio::process::Command;
use nix::unistd::Pid;
use nix::sys::{signal, signal::Signal};
use rocket_db_pools::{Connection, sqlx};
use rocket_db_pools::sqlx::Acquire;

use crate::structures::{Db, GameConfig, CompatTool, GameRuntime, GameHistory, HistoryType, HistoryGame};

struct IntermediateTimestamp {
    timestamp: Option<String>,
    game: i64,
    total_playtime: i64
}

pub async fn get_game_conf(db: &mut Connection<Db>, id: i64) -> Option<GameConfig> {
    let conn = db.acquire().await.ok()?;
    let row = sqlx::query!(
        "SELECT launch_config FROM subgames WHERE id = ?",
        id
    ).fetch_optional(conn)
    .await
    .ok()??;
    
    Some(json::serde_json::from_str::<GameConfig>(&row.launch_config).ok()?)
}

pub async fn get_compat_tool(db: &mut Connection<Db>, id: i64) -> Option<CompatTool> {
    let conn = db.acquire().await.ok()?;
    let rows = sqlx::query!(
        "SELECT ct.id, ct.name, ct.executable, ct.environment
        FROM subgames g
        JOIN compat_tools ct ON g.compat_tool = ct.id
        WHERE g.id = ?;",
        id
    ).fetch_optional(conn)
    .await
    .ok()??;

    Some(
        CompatTool { 
            id: rows.id,
            name: rows.name,
            executable: rows.executable,
            environment: json::serde_json::from_str::<HashMap<String, String>>(&rows.environment?).ok()? 
        }
    )
}

pub async fn add_to_history(db: &mut Connection<Db>, timestamp_start: i64, timestamp_end: i64, game: i64) -> Result<(), Box<dyn std::error::Error>>{
    // Only insert new row when last game session was more than 600s away (keep the table clean) 
    let conn = db.acquire().await?;
    sqlx::query!(
        r#"
        INSERT INTO history (id, game, timestamp_start, timestamp_end)
        VALUES (
            (
                SELECT id
                FROM history
                WHERE game = ?3
                  AND (?2 - timestamp_end) <= 600
                ORDER BY timestamp_end DESC
                LIMIT 1
            ),?3,?2,?1
        )
        
        ON CONFLICT(id) DO UPDATE SET
            timestamp_end = excluded.timestamp_end;
        "#,
        timestamp_end,
        timestamp_start,
        game
    ).execute(conn).await?;
    Ok(())
}

#[get("/status")]
async fn get_status(game_runtime: &State<Arc<GameRuntime>>) -> String{
    let game_runtime: Arc<GameRuntime> = game_runtime.inner().clone();
    format!("{{\"current_game\": {}, \"game_running\": {}, \"running_since\": {}}}", game_runtime.current_game.load(Ordering::Relaxed), game_runtime.game_running.load(Ordering::Relaxed), game_runtime.running_since.load(Ordering::Relaxed))
}

#[get("/terminate")]
async fn terminate(game_runtime: &State<Arc<GameRuntime>>) -> String{
    let game_runtime: Arc<GameRuntime> = game_runtime.inner().clone();
    if game_runtime.game_running.load(Ordering::SeqCst) {
       let pid = Pid::from_raw(game_runtime.pid.load(Ordering::Relaxed).try_into().unwrap());
       signal::kill(pid, Signal::SIGTERM).expect("Process should exist!");
    }
    game_runtime.game_running.store(false, Ordering::SeqCst);
    "".to_string()
}

#[get("/history?<scope>&<date>")]
async fn get_history( mut db: Connection<Db>, scope: &str, date: Option<String>) -> Option<json::Json<Vec<GameHistory>>> {
    let mut param = "now".to_string();
    if let Some(date) = date {
        param = date;
    }
    let intermediate_timestamps = match scope {
        "week" => Some(sqlx::query_as!(IntermediateTimestamp,"SELECT 
                date(timestamp_start, 'unixepoch') AS timestamp,
                game,
                SUM(timestamp_end - timestamp_start) AS total_playtime
            FROM history
            WHERE timestamp_start >= strftime('%s', ?1, '-7 days')
            AND timestamp_start <= strftime('%s', ?1, '+1 days')
            GROUP BY timestamp, game
            ORDER BY timestamp DESC;", param).fetch_all(&mut **db).await.ok()?),
        "month" => Some(sqlx::query_as!(IntermediateTimestamp, "SELECT strftime('%Y-%W', timestamp_start, 'unixepoch') AS timestamp, 
                game, 
                SUM(timestamp_end - timestamp_start) AS total_playtime 
            FROM history 
            WHERE strftime('%Y-%m', timestamp_start, 'unixepoch') = strftime('%Y-%m', ?1)
            GROUP BY timestamp, game 
            ORDER BY timestamp DESC;", param).fetch_all(&mut **db).await.ok()?),

        "year" => Some(sqlx::query_as!(IntermediateTimestamp, "SELECT 
                strftime('%m', timestamp_start, 'unixepoch') AS timestamp,
                game,
                SUM(timestamp_end - timestamp_start) AS total_playtime
            FROM history
            WHERE strftime('%Y', timestamp_start, 'unixepoch') = strftime('%Y', ?1)
            GROUP BY timestamp, game
            ORDER BY timestamp DESC;", param).fetch_all(&mut **db).await.ok()?),
        _ => None
    };
    
    let r#type = match scope {
        "day" => HistoryType::DAY,
        "month" => HistoryType::WEEK,
        "year" => HistoryType::MONTH,
        _ => HistoryType::DAY
    };
     
    let mut last_date = "".to_string();
    let mut current_obj: GameHistory = GameHistory {
        r#type: HistoryType::WEEK,
        date: "".to_string(),
        games: vec![],
    };
    let mut obj_list: Vec<GameHistory> = vec![];


    for timestamp in intermediate_timestamps? {
        let week = timestamp.timestamp?;
        let game = timestamp.game;
        let time = timestamp.total_playtime;
        if last_date == week {
            current_obj.games.push(HistoryGame{
                id: game,
                playtime: time
            });
        } else {
            if current_obj.date != "".to_string() {
                obj_list.push(current_obj);
            }
            current_obj = GameHistory{
                r#type: r#type.clone(),
                date: week.clone(),
                games: vec![HistoryGame{
                    id: game,
                    playtime: time
                }],
            };
            last_date = week;
        }
    }
    if current_obj.date != "".to_string() {
        obj_list.push(current_obj);
    }


    Some(json::Json(obj_list))
}

#[get("/launch?<id>")]
async fn launch_game(id: i64, game_runtime: &State<Arc<GameRuntime>>, mut db: Connection<Db>) -> String {
    println!("Starting Game!");
    let game_runtime: Arc<GameRuntime> = game_runtime.inner().clone();
    if !game_runtime.game_running.swap(true, Ordering::SeqCst) {
        println!("Validated that no other game is running!");
            if let Some(compat_tool) = get_compat_tool(&mut db, id).await && let Some(game_config) = get_game_conf(&mut db, id).await {
                let arguments: Vec<String> = vec![game_config.executable].iter().chain(game_config.arguments.iter()).cloned().collect();
                let environment = compat_tool.environment.into_iter().chain(game_config.environment);
                let game_start = Utc::now();
                let game_start_unix = Utc::now().timestamp();
                game_runtime.running_since.store(game_start_unix as isize, Ordering::Relaxed);
                game_runtime.current_game.store(id as isize, Ordering::Relaxed);

                if game_config.archive_file != "".to_string() {
                    //TODO: [IMPL ARCHIVE SYSTEM] mount game's Squashfs
                }
                println!("Starting Process: {} run {:?}", compat_tool.executable, arguments);
                println!("In working_directory: {}", game_config.working_directory);
                println!("With Environment: {:?}", environment);
                let child = Command::new(compat_tool.executable)
                    .current_dir(game_config.working_directory)
                    .arg("run")
                    .args(arguments)
                    .env("STEAM_COMPAT_DATA_PATH", game_config.game_prefix.clone())
                    .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", game_config.game_prefix)
                    .envs(environment)
                    .spawn();
                if let Ok(mut child) = child {
                    if let Some(pid) = child.id() {
                        game_runtime.pid.store(pid, Ordering::Relaxed);
                    } else {
                        println!("Unable to get process ID!");
                    }

                    child.wait().await.expect("Process should be running!"); // Wait for tha child
                                                                             // to exit
                                                                            
                    game_runtime.game_running.store(false, Ordering::SeqCst);
                    let playtime = ((Utc::now() - game_start).num_minutes() as f32) / 60.0;
                    let row = sqlx::query!(
                        "UPDATE subgames SET playtime = playtime + ?, last_launch = ? WHERE id = ?",
                        playtime, 
                        game_start_unix, 
                        id
                    ).execute(&mut **db)
                    .await;
                    if let Ok(_row) = row {
                        println!("Updated game stats!");

                    } else {
                        println!("Failed to update game stats!");
                    }

                    println!("Adding to history...");
                    add_to_history(&mut db, game_start.timestamp(), Utc::now().timestamp(), id).await.unwrap_or_else(|err|{
                        println!("ERROR! {}", err);
                        ()
                    });
                } else {
                    game_runtime.game_running.store(false, Ordering::SeqCst);
                }        
            } else {
                game_runtime.game_running.store(false, Ordering::SeqCst);
            }
            
    } else {
        return "{\"status\":\"FAILED: a game is already running!\"}".to_string();
    }
    "{\"status\": \"successful\"}".to_string()
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_status, terminate, launch_game, get_history]
}
