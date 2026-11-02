{
  lib,
  rustPlatform,
  pkg-config,
  openssl,
  fuse3,
  glib,
}:

rustPlatform.buildRustPackage {
  pname = "game_archive";
  version = "0.1.0";

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  DATABASE_URL="sqlite:game_archive_BUILD.sqlite";

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl fuse3 glib ];

  meta = {
    description = "Tool to play squashfs archived games";
    license = lib.licenses.mit;
    mainProgram = "game_archive";
  };
}
