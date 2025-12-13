use std::sync::{atomic::AtomicBool, atomic::AtomicIsize, Arc, atomic::AtomicU32};
use std::process::Command;
use rocket::fs::FileServer;
use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
use rocket::launch;

mod routes;
mod structures;
//mod database_helper;

use structures::*;

async fn run_migrations(pool: &sqlx::SqlitePool) {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS artworks (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             mime_type TEXT,
             blob BLOB,
             type TEXT,
             game INT NOT NULL,
             FOREIGN KEY('game')
                REFERENCES 'games'('id')
                ON DELETE CASCADE
                ON UPDATE CASCADE,
            UNIQUE (game, type)
        );
        CREATE TABLE IF NOT EXISTS subgame_covers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            mime_type TEXT NOT NULL,
            image BLOB NOT NULL,
            subgame INT NOT NULL,
            FOREIGN KEY('subgame')
                REFERENCES 'subgames'('id')
                ON UPDATE CASCADE
                ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS games (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS subgames (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            playtime REAL,
            last_launch INTEGER,
            is_archived BOOL NOT NULL DEFAULT false,
            launch_config TEXT NOT NULL DEFAULT '{"arguments": [], "working_directory":"", "game_prefix":"", "executable":"", "environment":{}, "archive_file": ""}',
            compat_tool INT,
            parent INT NOT NULL,
            FOREIGN KEY ('compat_tool')
                REFERENCES 'compat_tools'('id')
                ON UPDATE CASCADE,
            FOREIGN KEY ('parent')
                REFERENCES 'games'('id')
                ON UPDATE CASCADE
                ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS compat_tools (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            executable TEXT NOT NULL,
            environment TEXT
        );
        CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_start INT NOT NULL,
            timestamp_end INT NOT NULL,
            game INT NOT NULL,
            FOREIGN KEY('game') 
                REFERENCES 'games'('id')
                ON UPDATE CASCADE
                ON DELETE CASCADE
        );"#
    )
    .execute(pool)
    .await
    .expect("Migration Failed!");
}

fn open_url(url: &str) {
    // Run the xdg-open command to open the URL in the browser
    let result = Command::new("xdg-open")
        .arg(url)
        .output();

    match result {
        Ok(_) => println!("Successfully opened the browser with the URL: {}", url),
        Err(e) => eprintln!("Failed to open the browser: {}", e),
    }
}

#[launch]
fn rocket() -> _ {
    let runtime = Arc::new(GameRuntime{
        game_running: AtomicBool::new(false),
        current_game: AtomicIsize::new(-1),
        running_since: AtomicIsize::new(0),
        pid: AtomicU32::new(0),
    });
    let config = rocket::config::Config::default();
    let url = format!("http://{}:{}/static/index.html", config.address, config.port);
    //open_url(&url);

    rocket::build()
        .attach(Db::init())
        .attach(AdHoc::on_ignite("Run Migrations", |rocket| async {
            let db_pool = Db::fetch(&rocket).unwrap();
            run_migrations(db_pool).await;
            rocket
        }))
        .manage(runtime)
        .mount("/api", routes::game::routes())
        .mount("/api", routes::media::routes())
        .mount("/api", routes::backend_launch::routes())
        .mount("/api", routes::game_config::routes())
        .mount("/static", FileServer::from("static"))
}
