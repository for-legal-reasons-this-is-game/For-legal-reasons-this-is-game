{
  description = "Rust dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    fenix,
    ...
  }: let
    # Single-machine setup. To support more, wrap the below in a helper over
    # a list of systems (e.g. nixpkgs.lib.genAttrs).
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    fpkgs = fenix.packages.${system};

    # ╭──────────────────────────────────────────────────────────────╮
    # │ EDIT PER PROJECT                                              │
    # ╰──────────────────────────────────────────────────────────────╯
    # Channel: "stable" | "beta" | "complete" (= nightly).
    # Use "complete" for custom JSON targets / -Z build-std / no_std.
    channelName = "stable";

    # Prebuilt std for KNOWN cross targets, e.g.
    #   ["i686-unknown-linux-gnu" "wasm32-unknown-unknown" "thumbv7em-none-eabihf"]
    # For a CUSTOM .json target leave empty and use build-std (.cargo/config.toml).
    extraTargets = [];
    # ╰──────────────────────────────────────────────────────────────╯

    channel = fpkgs.${channelName};

    toolchain = fpkgs.combine ([
        (channel.withComponents [
          "cargo"
          "clippy"
          "rustc"
          "rustfmt"
          "rust-src" # needed by rust-analyzer and for build-std
          "rust-analyzer" # LSP matched to this exact toolchain
        ])
      ]
      ++ map (t: fpkgs.targets.${t}.${channelName}.rust-std) extraTargets);
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = [
        toolchain
        pkgs.pkg-config
        pkgs.gcc # C toolchain + default linker
        pkgs.lld # faster / cross-friendly linker (optional)
      ];

      # So rust-analyzer can find std sources (completion, no_std, build-std).
      RUST_SRC_PATH = "${channel.rust-src}/lib/rustlib/src/rust/library";

      shellHook = ''
        echo "$(rustc --version)  |  $(cargo --version)"
      '';
    };

    formatter.${system} = pkgs.alejandra;
  };
}
