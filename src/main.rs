mod lane;
mod nostr;
mod ui;
mod win;

use std::sync::Arc;

use directories::ProjectDirs;
use nostr_sdk::Client;
use relm4::*;
use sqlx::SqlitePool;

#[derive(Debug)]
pub struct Gnostique {
    pool: Arc<SqlitePool>,
    dirs: ProjectDirs,
    client: Client,
}

fn main() {
    let app = RelmApp::new("com.jirijakes.gnostique");

    let settings = gtk::Settings::default().unwrap();
    settings.set_gtk_application_prefer_dark_theme(true);

    app.run_async::<crate::win::Win>(());
}
