use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};
use std::fs::File;
use tokio;
use uuid::Uuid;

use rust_apns_core::{
    client::{client::Client, Endpoint},
    notification::AlertNotificationBuilder,
    notification::PushNotification,
};

// An example client connectiong to APNs with a JWT token
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();

    let mut key_file = String::new();
    let mut team_id = String::new();
    let mut key_id = String::new();
    let mut device_token = String::new();
    let mut message = String::from("Ch-check it out!");
    let mut sandbox = false;
    let mut topic: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("APNs token-based push");
        ap.refer(&mut key_file)
            .add_option(&["-p", "--pkcs8"], Store, "Private key PKCS8");
        ap.refer(&mut team_id)
            .add_option(&["-t", "--team_id"], Store, "APNs team ID");
        ap.refer(&mut key_id)
            .add_option(&["-k", "--key_id"], Store, "APNs key ID");
        ap.refer(&mut device_token)
            .add_option(&["-d", "--device_token"], Store, "APNs device token");
        ap.refer(&mut message)
            .add_option(&["-m", "--message"], Store, "Notification message");
        ap.refer(&mut sandbox)
            .add_option(&["-s", "--sandbox"], StoreTrue, "Use the development APNs servers");
        ap.refer(&mut topic)
            .add_option(&["-o", "--topic"], StoreOption, "APNS topic");
        ap.parse_args_or_exit();
    }

    // Read the private key from disk
    let mut private_key = File::open(key_file).unwrap();

    // Which service to call, test or production?
    let endpoint = if sandbox {
        Endpoint::Development
    } else {
        Endpoint::Production
    };

    // Connecting to APNs
    let client = Client::token(&mut private_key, key_id, team_id, endpoint).unwrap();

    // Notification payload
    let alert = PushNotification::Alert(
        AlertNotificationBuilder::default()
            .body(message)
            .sound("default")
            .badge(1u32)
            .build()
            .unwrap(),
    );

    let payload = alert.build_request(topic, None, device_token, Uuid::new_v4()).unwrap();
    let response = client.send(payload).await?;

    println!("Sent: {:?}", response);

    Ok(())
}
