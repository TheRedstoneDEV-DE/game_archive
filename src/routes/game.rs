#[put("/games?edit&<subgame>", format = "json", data = "<data>")]
fn edit_game(subgame: bool, db: &State<Arc<DbConnection>>, data: json::Json<MetaGame>) -> String {
    let conn = db.0.lock().unwrap();
    if subgame {
        let rows_updated = conn.execute(
            "UPDATE games SET name = ?1, playtime = ?2, last_launch = ?3, is_archived = ?4 WHERE id = ?5",
            params![data.name, data.playtime, data.last_launch, data.archived, data.id]
        ).unwrap_or_else(|_| {
            0
        });
        if rows_updated == 0 {
            return "Failed to update Row!".to_string();
        }
        "Updated DB row successfully!".to_string()

    } else {
        let rows_updated = conn.execute(
            "UPDATE games SET name = ?1 WHERE id = ?2 AND is_subgame = true",
            params![data.name, data.id]
        ).unwrap_or_else(|_| {
            0
        });
        if rows_updated == 0 {
            return "Failed to update Row!".to_string();
        }
        "Updated DB row successfully!".to_string()
    }
}

#[get("/get_info?<id>")]
fn get_gameinfo(id: i64, db: &State<Arc<DbConnection>>) -> Option<json::Json<MetaGame>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, name, playtime, last_launch, is_archived FROM games WHERE games.id = ?1").ok()?;
    let game: MetaGame = stmt.query_row(params![id], |row|{
        let id = row.get::<usize, i64>(0)?;
        let name = row.get::<usize, String>(1)?;
        let playtime = row.get::<usize, f32>(2)?;
        let last_launch = row.get::<usize, i64>(3)?;
        let is_archived = row.get::<usize, bool>(4)?;
        Ok(MetaGame {
            id: id as u32,
            name: name,
            playtime: playtime,
            last_launch: last_launch,
            archived: is_archived
        })
        
    }).ok()?;
    Some(json::Json(game))
}

#[get("/games?<id>")]
fn get_game(id: i64, db: &State<Arc<DbConnection>>) -> json::Json<Game> {
    let conn = db.0.lock().unwrap();
    let mut statement = conn.prepare("SELECT id, name FROM games WHERE games.is_subgame = 0 AND games.id = ?1").expect("Should be able to prepare Statement!");
    let mut statement_subgame = conn.prepare("SELECT * FROM games WHERE games.is_subgame = 1 AND games.related_to = ?1").expect("Should be able to find all meta-games!");
    let mut games = statement.query_map(params![id], |row| {
        Ok(Game{
            id: row.get(0).unwrap(),
            name: row.get(1).unwrap(),
            sub_games: vec![]
        })
    }).expect("Should be able to find all values!");
   
    if let Some(Ok(game)) = games.next() {
        let subgames_iter = statement_subgame.query_map(params![game.id], |row| {
            Ok(SubGame {
                id: row.get(0).unwrap(),
                name: row.get(1).unwrap(),
                playtime: row.get(4).unwrap(),
                last_launch: row.get(5).unwrap(),
                archived: row.get(6).unwrap()
            })
        }).expect("Should be able to find all values");
        let subgames = subgames_iter.map(|g| g.unwrap()).collect();
        json::Json(Game{
            id: game.id,
            name: game.name,
            sub_games: subgames
        })
    } else {
        json::Json(Game{
            id: 0,
            name: "Error".to_string(),
            sub_games: vec![]
        })
    }
}

#[get("/games")]
fn get_games(db: &State<Arc<DbConnection>>) -> json::Json<Vec<MetaGame>> {
    let conn = db.0.lock().unwrap();
    let mut statement = conn.prepare("SELECT id, name FROM games WHERE games.is_subgame = 0").expect("Should be able to find all meta-games!");
    let mut statement_subgame = conn.prepare("SELECT * FROM games WHERE games.is_subgame = 1 AND games.related_to = ?1").expect("Should be able to find all meta-games!");
    let mut meta_games: Vec<MetaGame> = Vec::new();

    let games = statement.query_map([], |row| {
         Ok(Game {
             id: row.get(0).unwrap(),
             name: row.get(1).unwrap(),
             sub_games: vec![]
         })
    }).expect("Should be able to find all values");

    for game in games {
        let game = game.unwrap();
        let subgames = statement_subgame.query_map(params![game.id], |row| {
            Ok(SubGame {
                id: row.get(0).unwrap(),
                name: row.get(1).unwrap(),
                playtime: row.get(4).unwrap(),
                last_launch: row.get(5).unwrap(),
                archived: row.get(6).unwrap()
            })
        }).expect("Should be able to find all values");
        
        let mut total_playtime = 0.0;
        let mut last_launch = 0;
        let mut archived = false;

        for sg in subgames {
            let sg = sg.unwrap();
            total_playtime += sg.playtime;
            if sg.last_launch > last_launch {
                last_launch = sg.last_launch;
            }
            if sg.archived {
                archived = true;
            }
        }

        meta_games.push(MetaGame{
            id: game.id,
            name: game.name,
            playtime: total_playtime,
            last_launch: last_launch,
            archived: archived,
        });
    }
    json::Json(meta_games)
}

#[put("/games?add", format="json", data="<data>")]
fn put_games(data: json::Json<Game>, db: &State<Arc<DbConnection>>) -> String {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO games(name, is_subgame) VALUES (?1, ?2)",
        params![data.name, false]
    ).expect("Should be able to add head-game");
    let parent_id = conn.last_insert_rowid();
    for game in &data.sub_games {
        conn.execute(
            "INSERT INTO games(name, is_subgame, related_to, playtime, last_launch, is_archived) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![game.name, true, parent_id, game.playtime, game.last_launch, game.archived]
        ).expect("Should be able to add Subgames!");
    }
    "DB was updated successfully!".to_string()
}

#[put("/games?<id>", format="json", data="<data>")]
fn put_games_id(id: i32, data: json::Json<GameConfig>) -> String {
    let json_content = serde_json::to_string_pretty(&*data).expect("It should be a valid object!");
    let mut file = File::create(format!("data/game{}.json",id)).expect("[data/game<id>.json]");
    file.write_all(json_content.as_bytes()).expect("[data/game<id>.json] Should be able to write to created file!");
    "File updated successfully!".to_string()
}

pub fn routes() -> Vec<rocket:Route> {
    routes![edit_game, get_game, get_games, get_gameinfo, put_games, put_games_id]
}
