use std::path::Path;
use damascus::{Filesystem, FuseOverlayFs, StateRecovery};
use std::fs;
use dirs::config_dir;

pub fn mount_overlayfs() {
    let config_dir = config_dir().expect("Error locating config dir!").join("game_archive");
    let lower_ro = config_dir.join("mount/read-only");
    let overlay = config_dir.join("mount/overlay");
    let tmp_dir = Path::new("/tmp/game_archive");
    let upper = tmp_dir.join("upper");
    let work = tmp_dir.join("work");

    if let Ok(mut recovered) = FuseOverlayFs::recover(&overlay) {
        recovered.unmount().expect("Failed to unmount overlay!");
    }

    if !tmp_dir.exists() {
        fs::create_dir_all(&upper).expect("Failed to create tempdir mount!");
        fs::create_dir(&work).expect("Failed to create overlayfs workdir!");
    }

    let mut o = FuseOverlayFs::writable([&lower_ro], upper, work, overlay).unwrap();
    o.set_scoped(false);
    o.mount().unwrap();
} 
