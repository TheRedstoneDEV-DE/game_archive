use std::path::Path;

use game_archive::mount_helper;

#[tokio::main]
#[test]
async fn main() {
    let _background_handle = mount_helper::mount_squashfs(Path::new("/run/media/robert/8cc05308-96c6-41f9-8bd3-1dd4c8b907df/ME-Andromeda.squashfs")).await;
    let _overlay_handle = mount_helper::mount_overlayfs(); 
    tokio::signal::ctrl_c().await.expect("Failed to wait for CTRL-C...")
}
