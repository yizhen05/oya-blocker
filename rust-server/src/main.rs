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

// ----------------------------------------------------
// ★★ 監視したい自分のDiscordユーザーIDを設定 ★★
const TARGET_USER_ID: u64 = 964540731345743893; 
// ----------------------------------------------------

/// M5Stackに返すJSONの型
#[derive(Clone, Debug, Serialize, Deserialize)]
struct VoiceStatus {
    status: String,
}

/// BotとServerで共有するアプリケーションの状態
/// RwLock (Read/Write Lock) で排他制御を行う
type AppState = Arc<RwLock<VoiceStatus>>;

// --- Discord Bot (Serenity) ---

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    /// VCの状態が変化したときに呼ばれる
    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        // 更新がターゲットユーザーのものかチェック
        // (注: 退出時は 'new' に user_id がない場合があるので 'old' も見る)
        let user_id = new.user_id.get();
        if user_id != TARGET_USER_ID {
            if let Some(ref old_state) = old {
                if old_state.user_id.get() != TARGET_USER_ID {
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
        let state_lock = ctx.data.read().await;
        let shared_state = state_lock
            .get::<AppStateKey>()
            .expect("Failed to get AppState");

        let mut status = shared_state.write().await;
        status.status = new_status_str.to_string();
    }

    /// Bot起動時に呼ばれる
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("Discord Bot {} is connected!", ready.user.name);

        // 起動時にVCに参加しているかチェック (Python版の check_initial_vc_status と同じ)
        let mut initial_status = "off";
        let target_user_id = UserId::new(TARGET_USER_ID);

        // Botが参加している全サーバーをチェック
        for guild_id in ctx.cache.guilds() {
            if let Some(guild) = ctx.cache.guild(guild_id) {
                // サーバーのボイス状態マップにターゲットユーザーがいるか
                if let Some(voice_state) = guild.voice_states.get(&target_user_id) {
                    if voice_state.channel_id.is_some() {
                        tracing::info!("Initial check: User is already in VC.");
                        initial_status = "on";
                        break; // 見つかったらループを抜ける
                    }
                }
            }
        }

        // 共有ステータスを更新
        let state_lock = ctx.data.read().await;
        let shared_state = state_lock
            .get::<AppStateKey>()
            .expect("Failed to get AppState");
        
        let mut status = shared_state.write().await;
        status.status = initial_status.to_string();
    }
}

// SerenityのContextにAppStateを格納するためのキー
struct AppStateKey;
impl TypeMapKey for AppStateKey {
    type Value = AppState;
}

/// Discord Botタスク
async fn run_discord_bot(shared_state: AppState) {
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");

    // VC状態とサーバーメンバーを監視するインテントを有効化
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
) -> Json<VoiceStatus> {
    // 共有ステータスを読み取りロックして、クローンを返す
    let status = state.read().await;
    Json(status.clone())
}

/// Webサーバータスク
async fn run_web_server(shared_state: AppState) {
    let app = Router::new()
        .route("/status", get(get_status_handler))
        .with_state(shared_state); // ハンドラに共有ステータスを渡す

    // M5Stackからアクセスできるよう 0.0.0.0 (全インターフェース) で待機
    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    tracing::info!("Web server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// --- Main (Tokio) ---

#[tokio::main]
async fn main() {
    // トレース（ログ）の初期化
    tracing_subscriber::fmt::init();

    // 1. 共有ステータスを初期化
    let shared_state = Arc::new(RwLock::new(VoiceStatus {
        status: "off".to_string(), // 初期値
    }));

    // 2. WebサーバーとDiscord Botを並行実行
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