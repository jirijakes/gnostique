#![feature(option_result_contains)]

mod lane;
mod nostr;
mod ui;
mod win;

use nostr_sdk::nostr::prelude::*;
// use nostr_sdk::nostr::util::time::timestamp;
use nostr_sdk::*;
use relm4::*;

#[tokio::main]
async fn main() -> Result<()> {
    let secret_key =
        SecretKey::from_bech32("nsec1qh685ta6ht7emkn8nlggzjfl0h58zxntgsdjgxmvjz2kctv5puysjcmm03")
            .unwrap();

    // npub1mwe5spuec22ch97tun3znyn8vcwrt6zgpfvs7gmlysm0nqn3g5msr0653t
    let keys = Keys::new(secret_key);

    // std::fs::create_dir_all("store")?;
    let client = Client::new(&keys);

    // client.restore_relays().await?;

    // npub1gl23nnfmlewvvuz7xgrrauuexx2xj70whdf5yhd47tj0r8p68t6sww70gt

    // client.connect().await;

    // client.sync().await?;

    // let sub = SubscriptionFilter::new()
    //     // .pubkey(XOnlyPublicKey::from_bech32(
    //     // "npub1gl23nnfmlewvvuz7xgrrauuexx2xj70whdf5yhd47tj0r8p68t6sww70gt",
    //     // )?)
    //     .events(vec![
    //         "b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"
    //             .parse()
    //             .unwrap(),
    //     ])
    //     // .limit(20)
    //     ;

    // client
    //     .subscribe(vec![
    //         sub,
    //         SubscriptionFilter::new()
    //             .id("b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"),
    //     ])
    //     .await?;

    // let x = client
    //     .get_events_of(vec![
    //         sub,
    //         SubscriptionFilter::new()
    //             .id("b4ee4de98a07d143f989d0b2cdba70af0366a7167712f3099d7c7a750533f15b"),
    //     ])
    //     .await?;

    // x.iter().for_each(|a| println!("{}", a.as_json().unwrap()));

    let app = RelmApp::new("com.jirijakes.gnostique");

    let settings = gtk::Settings::default().unwrap();
    settings.set_gtk_application_prefer_dark_theme(true);

    app.run_async::<crate::win::Gnostique>(client);

    Ok(())
}
