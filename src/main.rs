use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::{Hash, Hasher},
    thread,
    time::Duration,
};

use anyhow::Context;
use scraper::{Html, Selector};
use twilio::{Client, OutboundMessage};

const URL: &str = "https://www.schoolspring.com/jobs/?employer=12687";

const CHECK_DURATION: Duration = Duration::from_secs(60);

const FROM: &str = "+17812085883";

fn main() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(start())
}

async fn start() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let twilio = Client::new(
        &env::var("TWILIO_SID").context("TWILIO_SID env var required")?,
        &env::var("TWILIO_TOKEN").context("TWILIO_TOKEN env var required")?,
    );

    let to = env::var("TO").context("TO env var required")?;

    let mut previous_hash = None;

    loop {
        println!("Waiting {} seconds", CHECK_DURATION.as_secs_f64());
        thread::sleep(CHECK_DURATION);

        let response = match reqwest::get(URL).await {
            Err(_) => {
                eprintln!("Failed to fetch {}... Skipping", URL);
                continue;
            }
            Ok(resp) => resp,
        };

        if !response.status().is_success() {
            eprintln!("Server returned error code: {}", response.status());
        }

        let html = response
            .text()
            .await
            .context("Failed to get text content of the response")?;

        let html = Html::parse_document(&html);

        let body = {
            let selector = Selector::parse("body > table:nth-child(5) > tbody:nth-child(1) > tr:nth-child(2) > td:nth-child(1) > table:nth-child(1)").unwrap();

            match html.select(&selector).next() {
                Some(table) => table.text().collect::<Vec<_>>(),
                None => {
                    eprintln!("Table is missing!!! Trying again...");

                    twilio
                        .send_message(OutboundMessage::new(
                            FROM,
                            &to,
                            &format!("Ran into unexpected error: Table has gone missing\n{}", URL),
                        ))
                        .await?;

                    continue;
                }
            }
        };

        let hash = {
            let mut hasher = DefaultHasher::new();

            body.hash(&mut hasher);

            hasher.finish()
        };

        if let Some(previous_hash) = previous_hash {
            if previous_hash != hash {
                println!("UPDATE: {} != {}", previous_hash, hash);

                twilio
                    .send_message(OutboundMessage::new(
                        FROM,
                        &to,
                        &format!("NEW POSITIONS AVAILABLE: \n{}", URL),
                    ))
                    .await?;
            } else {
                print!("nothing...")
            }
        } else {
            print!("Got first hash: {}...", hash);
        };

        previous_hash = Some(hash);
    }
}
