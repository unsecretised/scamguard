use dotenvy::dotenv;
use img_hash::HasherConfig;
use poise::serenity_prelude::{self as serenity, Error};

const IMAGE_1: &[u8] = include_bytes!("../data/scam.jpg");
const IMAGE_2: &[u8] = include_bytes!("../data/scam1.jpg");

struct AppData {}

type Context<'a> = poise::Context<'a, AppData, Error>;

#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::all();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![age()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(AppData {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, AppData, Error>,
    _: &AppData,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } => {
            if new_message.attachments.is_empty() {
                return Ok(());
            }

            for attachment in new_message.attachments.clone() {
                let Ok(download) = attachment.download().await else {
                    return Ok(());
                };

                let is_scam = {
                    let hasher = HasherConfig::new().to_hasher();
                    let downloaded_image_hash = image::load_from_memory(&download)
                        .map_err(|_| Error::Other("Failed to load image"))?;
                    let download_hash = hasher.hash_image(&downloaded_image_hash);
                    let known_hash = hasher.hash_image(&image::load_from_memory(IMAGE_1).unwrap());
                    let known_hash_1 = hasher.hash_image(&image::load_from_memory(IMAGE_2).unwrap());
                    download_hash.dist(&known_hash) <= 10 || download_hash.dist(&known_hash_1) <= 10
                };

                if is_scam {
                    new_message.channel_id.say(ctx, "Scam detected").await?;
                }
            }

            Ok(())
        }

        _ => Ok(()),
    }
}
