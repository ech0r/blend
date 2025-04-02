{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, ...  }: 
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = true;
          };
        in
        with pkgs;
        {
          devShells.default = mkShell {
            buildInputs = with pkgs; [
              # Use a stable Rust with specific components
              (rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" "rust-analyzer" ];
                targets = [ 
                  "thumbv7em-none-eabihf"
                  "thumbv6m-none-eabi"
                  "wasm32-unknown-unknown"
                ];
              })
            ] ++ [
              # System dependencies
              pkg-config
              openssl
              openssl.dev
              
              # WebRTC dependencies
              libsodium
              glib
              glib.dev
              libuv
              
              # Other dev tools
              alsa-lib
              bashInteractive
              cargo-generate
              cargo-make
              clippy
              elf2uf2-rs
              flip-link
              gdb
              libGL
              libudev-zero
              libxkbcommon
              lldb_18
              minicom
              openocd
              picotool
              pre-commit
              probe-rs
              rustfmt
              rustup
              udev
              vscode
              vulkan-loader
              wasm-pack
              wayland
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXrandr
            ];
            
            # Add critical environment variables for linking
            shellHook = ''
              export SHELL=/run/current-system/sw/bin/bash
              
              # OpenSSL configuration (critical for linking)
              export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
              export OPENSSL_DIR="${pkgs.openssl.dev}"
              export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
              export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
              
              # For WebRTC/TURN server
              export LD_LIBRARY_PATH="${pkgs.openssl.out}/lib:${pkgs.libsodium}/lib:$LD_LIBRARY_PATH"
              
              echo "Development environment ready with OpenSSL configured"
            '';
            
            # Make sure libraries are found during linking
            LD_LIBRARY_PATH = "${pkgs.openssl.out}/lib:${pkgs.libsodium}/lib";
          };
        }
    );
}
