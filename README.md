# oya-blocker (親ブロッカー)

Discordのボイスチャット（VC）に参加している間、M5Stackに「ONAIR」と表示するためのシステムです。



## 📝 概要

VC参加ステータスを監視するDiscord Botと、M5Stackからのリクエストに応答するWebサーバーを、Rust（`serenity` + `axum`）で同時に実行します。

M5StackはWi-Fi経由で定期的にRustサーバーにステータスを問い合わせ、その結果（`on` / `off`）に応じて画面表示を切り替えます。

* **サーバー**: `rust-server/` (Rust, Axum, Serenity)
* **クライアント**: `m5stack-client/` (C++, PlatformIO)

## 🛠 必要なもの

### ハードウェア
* M5Stack Basic (または Core / Core2)
* Wi-Fi環境 (サーバーPCとM5Stackが同じネットワークに接続できること)

### ソフトウェア
* [Rust と Cargo](https://www.rust-lang.org/tools/install)
* [VSCode](https://code.visualstudio.com/)
* [PlatformIO IDE (VSCode拡張機能)](https://platformio.org/install/ide?install=vscode)
* Discordアカウントと、Botを作成・サーバーに追加する権限

---

## 🚀 セットアップ手順

### 1. Discord Bot の準備

1.  [Discord Developer Portal](https://discord.com/developers/applications) にアクセスし、新しいApplicationを作成します。
2.  「Bot」タブでBotを作成し、**Token**をコピーしておきます。
3.  **Privileged Gateway Intents** セクションで、以下の2つを **ON** にします。
    * `SERVER MEMBERS INTENT`
    * `VOICE STATE INTENT`
    * (※ これがOFFだとVCの参加を検知できません)
4.  「OAuth2」>「URL Generator」タブで、`bot` スコープを選択し、Botを自分のサーバーに招待します。

### 2. サーバー側の設定 (Rust)

1.  `rust-server` ディレクトリに移動します。
    ```bash
    cd rust-server
    ```
2.  `.env.example` ファイルをコピーして `.env` ファイルを作成します。
    ```bash
    cp .env.example .env
    ```
3.  作成した `.env` ファイルを開き、Discord BotのTokenと、監視したい自分のDiscordユーザーIDを設定します。
    * `DISCORD_TOKEN`: ステップ1で取得したBotトークン。
    * `RUST_LOG`: `info` のままがおすすめです。
4.  `src/main.rs` を開き、`TARGET_USER_ID` 定数をあなたのDiscordユーザーID（数値）に書き換えます。
    ```rust
    // rust-server/src/main.rs
    // ----------------------------------------------------
    // ★★ 監視したい自分のDiscordユーザーIDを設定 ★★
    const TARGET_USER_ID: u64 = 123456789012345678; 
    // ----------------------------------------------------
    ```

### 3. サーバーPCのIPアドレス確認

M5Stackがアクセスするサーバー（Rustプログラムを実行するPC）のローカルIPアドレスを調べます。

* **Linux / macOS の場合:**
    ```bash
    hostname -I 
    # (例: 192.168.1.10)
    ```
* **Windows の場合:**
    ```bash
    ipconfig
    # (例: IPv4 アドレス . . . . . . . . . . . .: 192.168.1.10)
    ```

> **[重要]**
> このIPアドレスは、ルーターのDHCP設定で**固定化（静的IPリース）**することを強く推奨します。固定しない場合、PCを再起動するたびにIPアドレスが変わり、M5Stack側の設定 (`secret.ini`) も毎回修正する必要があります。

### 4. クライアント側の設定 (M5Stack)

1.  VSCodeで `m5stack-client` フォルダを **「フォルダを開く」** で開きます（PlatformIOが自動で認識します）。
2.  `secret.ini.example` ファイルをコピーして `secret.ini` ファイルを作成します。
3.  作成した `secret.ini` ファイルを開き、Wi-Fi情報と、ステップ3で調べたサーバーのIPアドレスを設定します。
    ```ini
    [secret]
    wifi_ssid = "YOUR_WIFI_SSID"
    wifi_password = "YOUR_WIFI_PASSWORD"
    server_url = "[http://192.168.1.10:5000/status](http://192.168.1.10:5000/status)" ; (← ステップ3のIPアドレス)
    ```

---

## 🏃 使い方 (Usage)

### Step 1: サーバーの起動

`rust-server` ディレクトリで、以下のコマンドを実行してBotとWebサーバーを起動します。

```bash
cd rust-server
cargo run --release
