# Nix パッケージング・dotfiles統合

## スタンドアロンFlake (`~/sprout/flake.nix`)

sproutリポジトリ自体がNix Flakeとして機能し、以下を提供する:

- `packages.default`: sprout CLIバイナリ (`rustPlatform.buildRustPackage`)
- `packages.kakoune-sprout`: Kakouneプラグイン (`kakouneUtils.buildKakounePluginFrom2Nix`)
- `devShells.default`: 開発環境 (cargo, rustc, rust-analyzer, clippy, rustfmt)

## dotfiles統合

### flake.nix にinputを追加

```nix
# dotfiles/flake.nix
sprout.url = "github:Yus314/sprout";  # または開発中は "path:/home/kaki/sprout"
sprout.inputs.nixpkgs.follows = "nixpkgs";
```

### applications/kakoune/default.nix にCLIとプラグインを追加

```nix
{ pkgs, inputs, ... }:
{
  home.packages = with pkgs; [
    kakoune-lsp
    nixd
    markdown-oxide
    inputs.sprout.packages.${pkgs.stdenv.hostPlatform.system}.default  # sprout CLI
  ];

  programs.kakoune = {
    plugins = [
      pkgs.kakounePlugins.kakoune-scrollback
      pkgs.kakounePlugins.kakoune-autothemes
      inputs.sprout.packages.${pkgs.stdenv.hostPlatform.system}.kakoune-sprout  # plugin
    ];
  };
}
```

### 代替案: 独立アプリケーションモジュール

`applications/tf-wrapper/default.nix` パターンに従い、`applications/sprout/default.nix` を作成して `homes/common.nix` でインポートする方法もある。

## ビルド検証

```bash
# Nixビルド
nix build ~/sprout
./result/bin/sprout --help

# 開発シェル
cd ~/sprout && nix develop
cargo build && cargo test
```
