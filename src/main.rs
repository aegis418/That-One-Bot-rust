use std::collections::{HashSet};
use std::env;
use std::sync::Arc;

use regex::*;

use songbird::SerenityInit;
use serenity::client::Context;
use reqwest::Client as HttpClient;

use serenity::{
    async_trait,
    client::{Client, EventHandler},
    framework::{
        StandardFramework,
        standard::{
            Args, CommandResult,
            macros::{command, group}
        }
    },
    http::Http,
    model::{channel::Message, gateway::Ready},
    prelude::*,
    Result as SerenityResult,
};
use serenity::builder::CreateMessage;
use serenity::framework::standard::Configuration;

use serenity::model::id::{GuildId};
use serenity::model::prelude::{VoiceState};

use tracing::{error, info};

use commands::{
    spins::*,
    tags::*,
    utility::*,
    voice::*,
};

mod apis;
mod util;
mod commands;

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, message: Message) {
        let re = Regex::new(r"^.*(ifunny.co/picture).*$").unwrap();
        if re.is_match(message.content.as_str()) {
            let url = ifunny_replace(&message);
            let mess = CreateMessage::new().content(url);
            let _ = message.channel_id.send_message(&ctx.http, mess).await;
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is ready.", ready.user.name);
    }

    async fn voice_state_update(&self, _ctx: Context, _old: Option<VoiceState>, _new: VoiceState) {
        // Do something when someone leaves.
        match _old {
            None => {}
            Some(state) => {
                match state.channel_id {
                    None => {}
                    Some(cid) => {
                        // Disconnect bot if it is the only connection left in a voice channel.
                        let channel = cid.to_channel(&_ctx.http).await.unwrap().guild().unwrap();
                        let members = channel.members(&_ctx.cache).unwrap();
                        // println!("OLD: {:?}", members);
                        if members.len() == 1 {
                            if members[0].user.bot {
                                songbird::get(&_ctx).await.unwrap().remove(channel.guild_id).await;
                            }
                        }
                    }
                }
            }
        }
        // Do something when someone connects.
        // _new
    }
}

#[group]
#[commands(get_avatar, boxes)]
struct General;

#[group]
#[commands(dan, yan, kona, safe, auto_spin)]
struct Spins;

#[group]
#[commands(tag)]
struct Tags;

#[group]
#[commands(join, leave, play, stop, now_playing)]
struct Voice;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load from .env file.");
    tracing_subscriber::fmt().init();

    // Get token from env var.
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token env variable");

    let http = Http::new(&token);

    let (owner, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owner = HashSet::new();
            owner.insert(info.owner.unwrap().id);
            (owner, info.id)
        }
        Err(why) => panic!("Could not access app info: {:?}", why),
    };


    let framework = StandardFramework::new()
        .group(&GENERAL_GROUP)
        .group(&SPINS_GROUP)
        .group(&TAGS_GROUP)
        .group(&VOICE_GROUP);
    framework.configure(Configuration::new().owners(owner).prefix(";"));

    let intents = GatewayIntents::non_privileged() | GatewayIntents::privileged();

    // Build the bot client
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Error creating client.");

    // Register NowPlaying into client global data.
    {
        let mut data = client.data.write().await;

        data.insert::<NowPlaying>(Arc::new(RwLock::new(NowPlaying::None)))
    }

    // Start the client.
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
