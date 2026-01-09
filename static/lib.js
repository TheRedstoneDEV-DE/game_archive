var getJSON = function(url, callback) {
    var xhr = new XMLHttpRequest();
    xhr.open('GET', url, true);
    xhr.responseType = 'json';
    xhr.onload = function() {
      var status = xhr.status;
      if (status === 200) {
        callback(null, xhr.response);
      } else {
        callback(status, xhr.response);
      }
    };
    xhr.send();
};

function parseIdFromUrl() {
  return parseInt(window.location.hash.replace("#id=", ""));
}

async function getJSONAsync(url) {
  var resp = await fetch(url,{method: "GET"});
  return await resp.json();
}

function getSubgameByID(game, id) {
  for (sub_game of game.subgames) {
    if (sub_game.id == id){
      return sub_game;
    }
  }
}

async function postImage(url, data){
  return await fetch(url, {
    method: "POST",
    body: data
  });
}

async function postJSON(url, json) {
  var response = await fetch(url, {
    method: "POST",
    body: json,
    headers: {
      "Content-type": "application/json; charset=UTF-8"
    }
  });

  return await response.json();
} 

async function populate_compat() {
  var elements = document.getElementsByName("compat_tool"); 
  var template = document.getElementsByTagName("template")[0];            //
  if (template != undefined) {                                            // In case there is a template element
    elements = [template.content.querySelector("[name='compat_tool']")];  // -> TODO: migrate add.html  to template elements
  }                                                                       //    
  for (selection of elements) {
    let tools = await getJSONAsync("/api/compat_tools")
    for (tool of tools) {
      const option = document.createElement('option');
      option.value = tool.id;
      option.textContent = tool.name;
      selection.appendChild(option);
    }
  }
}

function parseSubgame(subgame_el, gameID, last_launch, is_archived){
  var subgame_playtime = subgame_el.querySelector("[name='subgame_playtime']").valueAsNumber; 
  return {
    id: parseInt(subgame_el.querySelector("[name='subid']").content),
    name: subgame_el.querySelector("[name='subgame_name']").value,
    playtime: subgame_playtime === 0 ? null : subgame_playtime,
    last_launch: last_launch,
    is_archived: is_archived,
    parent: gameID
  };
}

function parseGameConf(subgame_el, archive_file) {
  return {
    arguments: subgame_el.querySelector("[name='subgame_args']").value.split(" "),
    working_directory: subgame_el.querySelector("[name='subgame_workdir']").value,
    game_prefix: subgame_el.querySelector("[name='subgame_winprefix']").value,
    executable: subgame_el.querySelector("[name='subgame_executable']").value,
    environment: toHashMap(subgame_el.querySelector("[name='subgame_env']").value),
    archive_file: archive_file
  }
}

function splitArguments(args) {
  outString = "";
  for (arg of args) {
    outString += arg + " ";
  }
  return outString;
}

function hashToString(hashMap) {
  outString = "";
  Object.entries(hashMap).forEach(([key, value]) => {
    if (outString != "") {
      outString+= ";";
    }
    outString += `${key},${value}`;
  });
  return outString;
}

function toHashMap(input) {
  const pairs = input.split(';');

  const hashMap = pairs.reduce((acc, pair) => {
    const [key, value] = pair.split(',');
    acc[key] = value;
    return acc;
  }, {});

  return hashMap;
}

function formatDate(date, locale){
  if (date != 0 && date != null){
    return new Intl.DateTimeFormat(locale, {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric'
    }).format(date*1000);
  }
  return "Never"
}

function timeSince(unixTimestamp) {
  // Convert timestamp (seconds) → milliseconds
  const past = unixTimestamp * 1000;
  const now = Date.now();
  const diffMs = now - past;

  // Convert to total seconds
  const totalSeconds = Math.floor(diffMs / 1000);

  // Break into h:m:s
  const hours   = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  // Format as HH:MM:SS with leading zeros
  return [
    String(hours).padStart(2, "0"),
    String(minutes).padStart(2, "0"),
    String(seconds).padStart(2, "0"),
  ].join(":");
}

function load_transition() {
  const body = document.getElementById("fade-overlay");

  // Force a reflow to ensure transition triggers
  void body.offsetWidth; // read layout property

  // Slight delay to be safe
  setTimeout(() => {
    body.classList.add("hidden");
  }, 10); // 10ms is enough
}
