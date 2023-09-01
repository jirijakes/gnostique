mod app;
mod config;
mod demand;
mod download;
mod follow;
mod gnostique;
mod identity;
mod incoming;
mod nostr;
mod ui;

use relm4::*;

fn main() {
    gtk::init().unwrap();

    let app = RelmApp::from_app(
        gtk::Application::builder()
            .application_id("com.jirijakes.gnostique")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build(),
    );

    // GTK and resources
    gtk::glib::set_application_name("Gnostique");
    gtk::gio::resources_register_include!("resources.gresource").unwrap();
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/com/jirijakes/gnostique/ui/style.css");
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        )
    };

    let icon_theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().unwrap());
    icon_theme.add_resource_path("/com/jirijakes/gnostique/icons");

    let settings = gtk::Settings::default().unwrap();
    settings.set_gtk_application_prefer_dark_theme(true);
    settings.set_gtk_xft_hinting(1);
    settings.set_gtk_xft_rgba(Some("rgb"));

    crate::app::action::make_app_actions().register_for_main_application();

    app.run::<crate::ui::app::App>(());
}
