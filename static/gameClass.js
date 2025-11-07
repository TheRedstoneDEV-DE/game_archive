class SubGame {
  constructor(id, name, playtime, last_launch, archived){
    this.id = id;
    this.name = name;
    this.playtime = playtime;
    this.last_launch = last_launch;
    this.archived = archived;
  }
}

class Game {
  constructor(id, name, subgames){
    this.id = id;
    this.name = name;
    this.sub_games = subgames;
  }
  static fromJSON(json) {
    const obj = typeof json === "string" ? JSON.parse(json) : json;
    return new Game(obj.id, obj.name, obj.subgames);
  }
}

class MetaGame {
  constructor(game) {
    this.id = game.id;
    this.name = game.name;
    // calculate total playtime ans last_launch
    this.playtime = 0;
    this.last_launch = 0;

    for (i in game.sub_games) {
      var current_game = game.sub_games[i];
      this.playtime += current_game.playtime;
      if (current_game.last_launch > this.last_launch){
        this.last_launch = current_game.last_launch;
      }
    }
  }
}

class CompatTool {
  constructor(id, name, executable, environment){
    this.id = id;
    this.name = name;
    this.executable = executable;
    this.environment = environment;
  }
}

class GameConfig {
  constructor(arguments2, working_directory, game_prefix, executable, environment, archive_file, compat_tool){
    this.arguments2 = arguments2;
    this.working_directory = working_directory;
    this.game_prefix = game_prefix;
    this.executable = executable;
    this.environment = environment;
    this.archive_file = archive_file;
    this.compat_tool = compat_tool;
  }
}
