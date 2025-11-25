# oya-blocker (親ブロッカー) for XIAO ESP32C3

Discordのボイスチャット（VC）に参加している間、Seeed Studio XIAO ESP32C3 を使って「ONAIR」ランプ（LEDやリレー）を点灯させるシステムです。

以前のM5Stack版から、より小型な **Seeed Studio XIAO ESP32C3** に移行し、物理的なライトの制御に特化しました。

## 📝 概要

VC参加ステータスを監視するDiscord Botと、マイコンからのリクエストに応答するWebサーバーを、Rust（`serenity` + `axum`）で同時に実行します。

XIAO ESP32C3はWi-Fi経由で定期的にRustサーバーにステータスを問い合わせ、その結果（`on` / `off`）に応じてGPIOピン（D0）の出力を切り替え、接続されたLEDやリレーを制御します。

* **サーバー**: `rust-server/` (Rust, Axum, Serenity)
* **クライアント**: `m5stack-client/` (C++, PlatformIO) ※フォルダ名は既存のままですが中身はXIAO用です

## 🛠 必要なもの

### ハードウェア
* **Seeed Studio XIAO ESP32C3**
* **制御したいデバイス**
   * LED（+ 抵抗）    * または リレーモジュール（100Vのライトなどを制御する場合）
* 配線用ケーブル、ブレッドボードなど
* Wi-Fi環境 (サーバーPCとXIAOが同じネットワークに接続できること)

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
3.  作成した `.env` ファイルを開き、Discord BotのTokenを設定します。
   * `DISCORD_TOKEN`: ステップ1で取得したBotトークン。    * `RUST_LOG`: `info` のままがおすすめです。
4.  `src/main.rs` を開き、`TARGET_USER_ID` 定数をあなたのDiscordユーザーID（数値）に書き換えます。
    ```rust
    // rust-server/src/main.rs (環境変数で指定する場合は.envのみでOK)
    // ----------------------------------------------------
    // ★★ 監視したい自分のDiscordユーザーIDを設定 ★★
    // 環境変数 TARGET_USER_ID で指定している場合はそのままで動作します
    // ----------------------------------------------------
    ```

### 3. サーバーPCのIPアドレス確認

XIAOがアクセスするサーバー（Rustプログラムを実行するPC）のローカルIPアドレスを調べます。

* **Linux / macOS の場合:** `hostname -I`
* **Windows の場合:** `ipconfig`

> **[重要]**
> このIPアドレスは、ルーターのDHCP設定で**固定化（静的IPリース）**することを強く推奨します。

### 4. クライアント側の設定 (XIAO ESP32C3)

#### 配線 (Wiring)
XIAO ESP32C3の **D0** ピン（GPIO 2）を制御ピンとして使用します。

* **LEDの場合:**
 * XIAO `D0` -> 抵抗 -> LEDのアノード(+)  * XIAO `GND` -> LEDのカソード(-)
* **リレーモジュールの場合:**
 * XIAO `D0` -> リレーの信号入力(IN)  * XIAO `5V` or `3V3` -> リレーのVCC
 * XIAO `GND` -> リレーのGND
#### PlatformIOの設定
1.  VSCodeで `m5stack-client` フォルダを **「フォルダを開く」** で開きます。
2.  `platformio.ini` が `board = seeed_xiao_esp32c3` になっていることを確認します（なっていなければ修正してください）。
3.  `secret.ini.example` ファイルをコピーして `secret.ini` ファイルを作成します。
4.  作成した `secret.ini` ファイルを開き、Wi-Fi情報と、ステップ3で調べたサーバーのIPアドレスを設定します。
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
```

### Step 2: クライアント(XIAO)への書き込み・起動

1.  XIAO ESP32C3をPCにUSB接続します。
2.  VSCode (PlatformIO) の左下の矢印アイコン（Upload）をクリックして、プログラムを書き込みます。
3.  書き込み完了後、シリアルモニタ（コンセントアイコン）を開くと、接続状況のログが確認できます。

### 動作確認

1.  XIAOが起動し、Wi-Fiに接続されると、定期的にサーバーへ状態を確認しに行きます。
2.  あなたがDiscordサーバーの **ボイスチャット(VC)に参加** します。
3.  数秒以内に、XIAOに接続された **LED/リレーがON** になります。
4.  VCから **退出** すると、**OFF** になります。