use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::model::{
    gateway::Ready,
    id::UserId,
    voice::VoiceState,
};
use serenity::prelude::*;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use dotenvy::dotenv;

// --- 共有ステートとレスポンスの定義 ---

/// BotとServerで共有するアプリケーションの状態
#[derive(Clone, Debug)]
struct SharedStatus {
    status: String,
    target_user_id: u64, // ★ IDを定数からステートに移動
}

/// M5Stackに返すJSONの型
#[derive(Clone, Debug, Serialize, Deserialize)]
struct VoiceStatusResponse {
    status: String,
}

/// 共有ステート全体 (Arc<RwLock<>>) の型エイリアス
type AppState = Arc<RwLock<SharedStatus>>;

// SerenityのContextにAppStateを格納するためのキー
struct AppStateKey;
impl TypeMapKey for AppStateKey {
    type Value = AppState;
}

// --- Discord Bot (Serenity) ---

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    /// VCの状態が変化したときに呼ばれる
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        
        // ★ 共有ステートから target_user_id を取得
        let state_lock = ctx.data.read().await;
        let shared_state_arc = state_lock
            .get::<AppStateKey>()
            .expect("Failed to get AppState")
            .clone(); // AppStateのArcをクローン
        
        let target_user_id = shared_state_arc.read().await.target_user_id; // IDを読み取り

        // 更新がターゲットユーザーのものかチェック
        let user_id = new.user_id.get();
        if user_id != target_user_id {
            if let Some(ref old_state) = old {
                if old_state.user_id.get() != target_user_id {
                    return; // ターゲットユーザーではない
                }
            } else {
                return; // ターゲットユーザーではない
            }
        }

        // 参加/退出の判定
        let is_joining = old.as_ref().and_then(|o| o.channel_id).is_none() && new.channel_id.is_some();
        let is_leaving = old.as_ref().and_then(|o| o.channel_id).is_some() && new.channel_id.is_none();

        let new_status_str = if is_joining {
            tracing::info!("User {} joined VC", user_id);
            "on"
        } else if is_leaving {
            tracing::info!("User {} left VC", user_id);
            "off"
        } else {
            return; // 参加/退出以外のイベント (ミュートなど) は無視
        };

        // 共有ステータスを書き込みロックして更新
        let mut status = shared_state_arc.write().await;
        status.status = new_status_str.to_string();
    }

    /// Bot起動時に呼ばれる
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("Discord Bot {} is connected!", ready.user.name);

        // ★ 共有ステートを取得
        let state_lock = ctx.data.read().await;
        let shared_state_arc = state_lock
            .get::<AppStateKey>()
            .expect("Failed to get AppState")
            .clone();

        let target_user_id_u64 = shared_state_arc.read().await.target_user_id;
        let target_user_id = UserId::new(target_user_id_u64); // serenity::model::id::UserId に変換

        // 起動時にVCに参加しているかチェック
        let mut initial_status = "off";

        for guild_id in ctx.cache.guilds() {
            if let Some(guild) = ctx.cache.guild(guild_id) {
                // サーバーのボイス状態マップにターゲットユーザーがいるか
                if let Some(voice_state) = guild.voice_states.get(&target_user_id) { // ★ 共有IDで検索
                    if voice_state.channel_id.is_some() {
                        tracing::info!("Initial check: User is already in VC.");
                        initial_status = "on";
                        break; 
                    }
                }
            }
        }

        // 共有ステータスを更新
        let mut status = shared_state_arc.write().await;
        status.status = initial_status.to_string();
    }
}


/// Discord Botタスク
async fn run_discord_bot(shared_state: AppState) {
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");

    let intents = GatewayIntents::GUILD_VOICE_STATES | GatewayIntents::GUILD_MEMBERS | GatewayIntents::GUILDS;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // BotのContext (ctx) に共有ステータス (AppState) を書き込む
    {
        let mut data = client.data.write().await;
        data.insert::<AppStateKey>(shared_state);
    }

    if let Err(why) = client.start().await {
        tracing::error!("Discord client error: {:?}", why);
    }
}

// --- Web Server (Axum) ---

/// M5Stackが /status にGETリクエストしたときのハンドラ
async fn get_status_handler(
    State(state): State<AppState>, // with_stateで渡された共有ステータス
) -> Json<VoiceStatusResponse> { // ★ M5Stack用のレスポンス型
    
    // status のみ読み取ってクローンする
    let status_str = state.read().await.status.clone();
    
    Json(VoiceStatusResponse {
        status: status_str
    })
}

/// Webサーバータスク
async fn run_web_server(shared_state: AppState) {
    let app = Router::new()
        .route("/status", get(get_status_handler))
        .with_state(shared_state); 

    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    tracing::info!("Web server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- Main (Tokio) ---

#[tokio::main]
async fn main() {
    // 1. .env ファイルから環境変数を読み込む
    dotenv().ok(); 
    
    // 2. トレース（ログ）の初期化
    tracing_subscriber::fmt::init();

    // ★ 3. .env から TARGET_USER_ID を読み込む
    let target_user_id_str = env::var("TARGET_USER_ID")
        .expect("Expected TARGET_USER_ID in environment");
    // 文字列からu64（数値）に変換
    let target_user_id = target_user_id_str
        .parse::<u64>()
        .expect("TARGET_USER_ID must be a valid u64 (number)");

    tracing::info!("Target User ID set to: {}", target_user_id);

    // 4. 共有ステータスを初期化 (IDもセット)
    let shared_state = Arc::new(RwLock::new(SharedStatus {
        status: "off".to_string(), 
        target_user_id: target_user_id, // ★ 読み込んだIDをセット
    }));

    // 5. WebサーバーとDiscord Botを並行実行
    tracing::info!("Starting services...");
    tokio::select! {
        _ = run_web_server(shared_state.clone()) => {
            tracing::error!("Web server task exited unexpectedly.");
        },
        _ = run_discord_bot(shared_state.clone()) => {
            tracing::error!("Discord bot task exited unexpectedly.");
        },
    }
}