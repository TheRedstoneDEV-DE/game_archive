use std::path::Path;
use damascus::{Filesystem, FuseOverlayFs};
use fuser::BackgroundSession;
use std::fs;
use dirs::config_dir;
use fuser_async::{FilesystemFUSE};
use squashfs_async::{Options, SquashFs, pools::LocalReadersPoolAsyncFs};

pub fn mount_overlayfs(images: usize) -> FuseOverlayFs {
    let config_dir = config_dir().expect("Error locating config dir!").join("game_archive");
    let lower_ro = config_dir.join("mount/read-only");
    let overlay = config_dir.join("mount/overlay");
    let tmp_dir = Path::new("/tmp/game_archive");
    let upper = tmp_dir.join("upper");
    let work = tmp_dir.join("work"); 

    if tmp_dir.exists() {
        // clean the overlay up in case a different image gets loaded
        fs::remove_dir_all(&upper).expect("Failed to clean up!");
        fs::remove_dir_all(&work).expect("Failed to clean up!");
    }

    fs::create_dir_all(&upper).expect("Failed to create tempdir mount!");
    fs::create_dir(&work).expect("Failed to create overlayfs workdir!");

    let mut lower_dirs = vec![];
    for image in 0..images {
        let lower_dir = lower_ro.join(image.to_string()); 
        lower_dirs.push(lower_dir.clone()); 
    }

    let mut o = FuseOverlayFs::writable(lower_dirs, upper, work, overlay).unwrap();
    // o.set_scoped(true); -> default value
    o.mount().unwrap();

    o // -> pass handle on, so it doesn't unmount right away
}

pub async fn mount_squashfs(file: &Path, index: usize) -> BackgroundSession {
    let mountpoint = config_dir().expect("Error locating config dir!").join("game_archive/mount/read-only").join(index.to_string());
    let mount_options = [
        fuser::MountOption::RO,
        fuser::MountOption::Async,
    ];
    
    let fs = FilesystemFUSE::new(
        SquashFs::<LocalReadersPoolAsyncFs>::open(&file, 
            &Options{
                cache_mb: 250, 
                readers: 4, 
                direct_limit: 0
            }
        ).await.expect("Failed to open Squashfs")
    );
    
    println!("Mounting game archive...");
    if !mountpoint.exists() {
        fs::create_dir(&mountpoint).expect("Failed to create lower directory!");
    }

    let mount = fuser::spawn_mount2(fs, &mountpoint, &mount_options).expect("... failed to mount archive!");
    println!("... done!");
    
    mount // -> pass handle on, so it doesn't unmount right away
}
