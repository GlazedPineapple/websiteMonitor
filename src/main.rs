use std::{env, time::Duration};

use async_std::task;
use color_eyre::eyre::{eyre, WrapErr};
use soup::prelude::*;
use twilio_async::{Twilio, TwilioRequest};

const URL: &str = "https://www.th3dstudio.com/product/ezout-filament-sensor-kit-standard/";
// const URL: &str = "https://www.th3dstudio.com/product/ezboard-lite/";

const CHECK_DURATION: Duration = Duration::from_secs(60);
const ALERT_DURATION: Duration = Duration::from_secs(15);

const FROM: &str = "+17812085883";

#[async_std::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    dotenv::dotenv().ok();

    let twilio = Twilio::new(
        env::var("TWILIO_SID").wrap_err("TWILIO_SID env var required")?,
        env::var("TWILIO_TOKEN").wrap_err("TWILIO_TOKEN env var required")?,
    )
    .wrap_err("Failed to setup twilio")?;

    loop {
        println!("Waiting {} seconds", CHECK_DURATION.as_secs_f64());
        task::sleep(CHECK_DURATION).await;

        let mut response = match surf::get(URL).await {
            Err(_) => {
                eprintln!("Failed to fetch {}... Skipping", URL);
                continue;
            }
            Ok(resp) => resp,
        };

        if !response.status().is_success() {
            eprintln!("Server returned error code: {}", response.status());
        }

        let body = response.body_string().await.map_err(|e| eyre!(e))?;

        // dbg!(body);

        let soup = Soup::new(&body);

        if soup
            .class("bundle_availability")
            .find()
            .unwrap()
            .class("stock")
            .class("out-of-stock")
            .find()
            .is_none()
        {
            print!("IN STOCK!!! ");

            for messages_left in (0u8..4u8).rev() {
                let fun_message = if messages_left == 0 {
                    "IM DONE YOU FUCKWIT, YOU BETTER HAVE BOUGHT IT :(".into()
                } else {
                    format!("{} more notifications before I off myself", messages_left)
                };

                twilio
                    .send_msg(
                        FROM,
                        &env::var("TO").wrap_err("TO env var required")?,
                        &format!("IN STOCK!!!!\n{}\n{}", fun_message, URL),
                    )
                    .run()
                    .await?;

                task::sleep(ALERT_DURATION).await;
            }

            break;
        } else {
            print!("OUT OF STOCK... ");
        }
    }

    Ok(())
}
