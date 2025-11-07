use rocket::{get, routes, State};
use rocket::serde::json;
use rusqlite::params;
use chrono::Utc;
use std::thread;
use std::sync::{atomic::Ordering, Arc};
use std::process::Command;
use nix::unistd::Pid;
use nix::sys::{signal, signal::Signal};

use crate::structures::*;
use crate::database_helper::{get_compat_tool, get_game_conf, add_to_history};

#[get("/status")]
fn get_status(game_runtime: &State<Arc<GameRuntime>>) -> String{
    let game_runtime: Arc<GameRuntime> = game_runtime.inner().clone();
    format!("{{\"current_game\": {}, \"game_running\": {}, \"running_since\": {}}}", game_runtime.current_game.load(Ordering::Relaxed), game_runtime.game_running.load(Ordering::Relaxed), game_runtime.running_since.load(Ordering::Relaxed))
}

#[get("/terminate")]
fn terminate(game_runtime: &State<Arc<GameRuntime>>) -> String{
    let game_runtime: Arc<GameRuntime> = game_runtime.inner().clone();
    if game_runtime.game_running.load(Ordering::SeqCst) {
       let pid = Pid::from_raw(game_runtime.pid.load(Ordering::Relaxed).try_into().unwrap());
       signal::kill(pid, Signal::SIGTERM).expect("Process should exist!");
    }
    game_runtime.game_running.store(false, Ordering::SeqCst);
    "".to_string()
}

#[get("/history?<scope>&<date>")]
fn get_history(db: &State<Arc<DbConnection>>, scope: &str, date: Option<String>) -> Option<json::Json<Vec<GameHistory>>> {
    let conn = db.0.lock().unwrap();
    let mut param = "now".to_string();
    if let Some(date) = date {
        param = date;
    }
    let sql = match scope {
        "week" => Some("SELECT 
                date(timestamp_start, 'unixepoch') AS day,
                game,
                SUM(timestamp_end - timestamp_start) AS total_playtime
            FROM history
            WHERE timestamp_start >= strftime('%s', ?1, '-7 days')
            AND timestamp_start <= strftime('%s', ?1, '+1 days')
            GROUP BY day, game
            ORDER BY day DESC;"),
        "month" => Some("SELECT strftime('%Y-%W', timestamp_start, 'unixepoch') AS week, 
                game, 
                SUM(timestamp_end - timestamp_start) AS total_playtime 
            FROM history 
            WHERE strftime('%Y-%m', timestamp_start, 'unixepoch') = strftime('%Y-%m', ?1)
            GROUP BY week, game 
            ORDER BY week DESC;"),
        "year" => Some("SELECT 
                strftime('%m', timestamp_start, 'unixepoch') AS month,
                game,
                SUM(timestamp_end - timestamp_start) AS total_playtime
            FROM history
            WHERE strftime('%Y', timestamp_start, 'unixepoch') = strftime('%Y', ?1)
            GROUP BY month, game
            ORDER BY month DESC;"),
        _ => None
    };
    
    let r#type = match scope {
        "day" => HistoryType::DAY,
        "month" => HistoryType::WEEK,
        "year" => HistoryType::MONTH,
        _ => HistoryType::DAY
    };
    // TODO: implement  filters (date, month, year)
    let mut stmt = conn.prepare(sql?).ok()?;
    let rows = stmt.query_map(params![param], |row| {
       let week = row.get::<usize, String>(0)?;
       let game = row.get::<usize, i64>(1)?;
       let time = row.get::<usize, i64>(2)?;
       Ok((week, game, time))
    }).ok()?;
    let mut last_date = "".to_string();
    let mut current_obj: GameHistory = GameHistory {
        r#type: HistoryType::WEEK,
        date: "".to_string(),
        games: vec![],
    };
    let mut obj_list: Vec<GameHistory> = vec![];


    for row in rows {
        let (week, game, time): (String, i64, i64) = row.ok()?;
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
fn launch_game(id: i64, game_runtime: &State<Arc<GameRuntime>>, db: &State<Arc<DbConnection>>) -> String {
    let game_runtime: Arc<GameRuntime> = game_runtime.inner().clone();
    let db: Arc<DbConnection> = db.inner().clone();
    if !game_runtime.game_running.swap(true, Ordering::SeqCst) {
        thread::spawn(move || {
            if let Some(compat_tool) = get_compat_tool(db.0.lock().unwrap(), id) && let Some(game_config) = get_game_conf(db.0.lock().unwrap(), id) {
                let arguments: Vec<String> = vec![game_config.executable].iter().chain(game_config.arguments.iter()).cloned().collect();
                let environment = compat_tool.environment.into_iter().chain(game_config.environment);
                let game_start = Utc::now();
                game_runtime.running_since.store(game_start.timestamp() as isize, Ordering::Relaxed);
                game_runtime.current_game.store(id as isize, Ordering::Relaxed);

                if game_config.archive_file != "".to_string() {
                    // mount game's Squashfs
                }
                let child = Command::new(compat_tool.executable)
                    .current_dir(game_config.working_directory)
                    .arg("run")
                    .args(arguments)
                    .env("STEAM_COMPAT_DATA_PATH", game_config.game_prefix.clone())
                    .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", game_config.game_prefix)
                    .envs(environment)
                    .spawn();
                if let Ok(mut child) = child {
                    game_runtime.pid.store(child.id(), Ordering::Relaxed);
                    child.wait().expect("Process should be running!");
                    game_runtime.game_running.store(false, Ordering::SeqCst);
                    let conn = db.0.lock().unwrap();
                    let playtime = ((Utc::now() - game_start).num_minutes() as f32) / 60.0;
                    conn.execute("UPDATE games SET playtime = playtime + ?1, last_launch = ?2 WHERE id = ?3",
                        params![playtime, game_start.timestamp(), id]
                    ).unwrap_or_else(|_|{
                      0 
                    });
                    println!("Adding to history...");
                    add_to_history(conn, game_start.timestamp(), Utc::now().timestamp(), id).unwrap_or_else(|err|{
                        println!("ERROR! {}", err);
                        ()
                    });
                } else {
                    game_runtime.game_running.store(false, Ordering::SeqCst);
                }        
            } else {
                game_runtime.game_running.store(false, Ordering::SeqCst);
            }
            
         });
    } else {
        return "{\"status\":\"FAILED: a game is already running!\"}".to_string();
    }
    "{\"status\": \"successful\"}".to_string()
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get_status, terminate, launch_game, get_history]
}
