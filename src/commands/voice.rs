use serenity::framework::standard::{Args, CommandResult, macros::command};
use serenity::model::prelude::*;
use serenity::prelude::*;
use songbird::tracks::TrackHandle;
use std::sync::Arc;

use regex::Regex;

use crate::apis::ocremix_api::*;
use std::ops::Deref;

// TODO: Expand on this using a hashmap to allow multiple guilds.
pub enum NowPlaying {
    None,
    Youtube {
        track: TrackHandle
    },
    OCRemix {
        track: TrackHandle,
        playing: OCRemix
    }
}

impl TypeMapKey for NowPlaying {
    type Value = Arc<RwLock<NowPlaying>>;
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.reply(&ctx.http, "Not in a voice channel").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx).await
        .expect("Did not init songbird in client builder.").clone();

    let _handler = manager.join(guild_id, connect_to).await;

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird not initialized").clone();

    if manager.get(guild_id).is_some() {
        if let Err(e) = manager.remove(guild_id).await {
            msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await?;
        }
    } else {
        msg.reply(&ctx.http, "Not in a voice channel").await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[sub_commands(play_ocremix)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            msg.channel_id.say(&ctx.http, "Must provide a URL or ID to a video or audio").await?;

            return Ok(());
        },
    };

    let re = Regex::new(r"(?m)^([a-zA-Z0-9_\-]{11,})$").unwrap();

    if !url.starts_with("http") && !re.is_match(&*url) {
        msg.channel_id.say(&ctx.http, "Must provide a valid URL").await?;

        return Ok(());
    }

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match songbird::ytdl(&url).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);

                msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await?;

                return Ok(());
            },
        };

        let track_handle = handler.play_source(source);

        let now_playing_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<NowPlaying>().expect("Expected NowPlaying in TypeMap.").clone()
        };

        // Update global now playing.
        {
            let mut now_playing = now_playing_lock.write().await;

            *now_playing = NowPlaying::Youtube { track: track_handle.clone() }

        }


        msg.channel_id.say(&ctx.http, "Playing song").await?;
    } else {
        msg.channel_id.say(&ctx.http, "Not in a voice channel to play in").await?;
    }

    Ok(())
}

#[command("ocremix")]
#[only_in(guilds)]
async fn play_ocremix(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let station: String = if !args.is_empty() {
        args.single::<String>().unwrap()
    } else {
        String::from("")
    };
    let station_id = StationID::from(station);
    let stream_url = station_id.get_stream_url().await;

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird not initialized").clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match songbird::ytdl(&*stream_url).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);

                msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await?;

                return Ok(());
            },
        };

        let track_handle = handler.play_source(source);

        let now_playing_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<NowPlaying>().expect("Expected NowPlaying in TypeMap.").clone()
        };

        // Update global now playing.
        {
            let mut now_playing = now_playing_lock.write().await;

            *now_playing = NowPlaying::OCRemix {
                track: track_handle.clone(),
                playing: get_current_song(station_id).await.unwrap()
            }

        }

    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird not initialized").clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let now_playing_lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<NowPlaying>().expect("Expected NowPlaying in TypeMap.").clone()
        };

        // Update global now playing.
        {
            let mut now_playing = now_playing_lock.write().await;

            *now_playing = NowPlaying::None;

        }

        handler.stop();

    } else {
        msg.reply(&ctx.http, "Not in a voice channel").await?;
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[aliases("np")]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let _guild_id = guild.id;

    let now_playing_lock = ctx.data.read().await;
    let now_playing = now_playing_lock.get::<NowPlaying>().expect("Expected NowPlaying in data").clone();

    {
        let now_playing =  now_playing.read().await;

        match now_playing.deref() {
            NowPlaying::None => {
                msg.channel_id.say(&ctx.http, "Nothing is playing").await?;
            }
            NowPlaying::Youtube { track } => {
                let metadata = track.metadata();
                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title(metadata.title.as_ref().unwrap());
                        e.url(metadata.source_url.as_ref().unwrap());
                        e.color(16741516)
                    })
                }).await?;
            }
            NowPlaying::OCRemix { playing, .. } => {
                let url = match playing.url.as_ref() {
                    None => {String::from("")}
                    Some(url) => {String::from(url)}
                };

                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.color(10276252);
                        e.title(&playing.title);
                        e.url(url);
                        e.thumbnail(&playing.album_url);
                        let station_name: &String = &playing.station_id.into();
                        e.description(format!("Album: {} \nStation: {}", playing.album, station_name))
                    })
                }).await?;
            }
        }

    }


    Ok(())
}