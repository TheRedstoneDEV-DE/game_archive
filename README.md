# The Game Archive
is an application that archives games into squashfs archives and launches them by mounting the archives (without needing to fully unpack them) in Linux.
It is meant for use with Windows games via Proton, but can also be used for native Linux titles. 
You can also launch non-archived games with it by using it as a normal game launcher. 
In terms of UI it is mostly built inspired by the Steam Bigpicture mode, meant to be usable on livingroom PCs (Gamepad controls are not implemented yet).

## Disclaimer
All games launched with this tool should be DRM-FREE (like for example games from GOG.com). 
I do not condone game piracy of any type and this project is not meant to be used with illegally obtained games.
This tool does not contain any sofware for removing DRM or other piracy related actions.

## How to use?
### Any Linux (Binary):
1. Install Dependencies:
- fuse3 (fusermount3)
- fuse-overlayfs
2. Download the latest statically-linked build from the releases section.
3. Execute the downloaded build:
```
./game_archive-x86_64-musl-static 
```
4. Visit the Web-UI at: http://localhost:8000
5. Add a compatibility tool by clicking on "Add" in the header bar
6. Add a game from the same menu

### Build from source
1. Install Dependencies:
- libfuse3 (fusermount3, development libs)
- fuse-overlayfs (development libs)
- rust-toolchain (recommended via rustup)
 OR (NixOS)
```
nix-shell
```
2. Build / Run
```
cargo run -r
```

## Configuring
All config files are located in your .config folder (home directory):
```
/home/[user]/.config/game_archive/
```
1. Editing the UI:
The UI comes bundled into the executable by default. 
Overrides can be done by placing the file into the `web-mixin` directory (in the config directory) with the same filename as in the static directory of this repo.
The hierarchy is the following:
```
dynamic API (/api/*) -> "web-mixin" direcory -> bundled
```

2. Editing the Rocket.toml
The Rocket.toml is also located in the config directory. Per default it describes the location of the GameArchive's Database (located in the config directory as well) which can also be changed there.
For other config options look up the Rocket.rs Rocket.toml documentation.

## Features:
- [x] Backend <done>
- - [x] Ability to archive games
- - [x] Ability to play archived games (mount, etc.)
- - [x] Track playtime of games (local DB)
- - [x] Track play sessions of games (local DB)
- [ ] Frontend
- - [x] Display games sorted by playtime, last-launch and alphabetically
- - [x] Launch games
- - [x] Add games + compatibility tools
- - [x] Configure games + compatibility tools
- - [ ] Visualize game sessions (with their playtime) [Mocked up]
- - [ ] Ability to remove compatibility tools
