{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  packages = [
    pkgs.rustup
    pkgs.pkgsCross.musl64.gcc
    pkgs.pkgsCross.musl64.binutils
    pkgs.pkgsCross.musl64.pkg-config
    pkgs.pkgsStatic.fuse-overlayfs
  ];

  buildInputs = [
    pkgs.pkgsStatic.zlib
    pkgs.pkgsStatic.zstd
    pkgs.pkgsStatic.fuse3
  ];

  shellHook = ''
    export CC_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-gcc
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-unknown-linux-musl-gcc

    export PKG_CONFIG_ALLOW_CROSS=1
  '';
}

