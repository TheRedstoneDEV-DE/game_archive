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
