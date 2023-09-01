use gtk::gdk;
use gtk::gio::SimpleActionGroup;
use gtk::prelude::DisplayExt;
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::AsyncComponentSender;

use crate::ui::main::{Main, MainInput};

/// Creates a GTK action group for app-scoped actions.
pub fn make_app_actions() -> RelmActionGroup<AppActionGroup> {
    let mut group = RelmActionGroup::<AppActionGroup>::new();

    group.add_action(copy_text());

    group
}

/// Copies a textual value into clipboard.
fn copy_text() -> RelmAction<Copy> {
    RelmAction::new_with_target_value(|_, string: String| {
        let display = gdk::Display::default().unwrap();
        let clipboard = display.clipboard();
        clipboard.set_text(&string);
    })
}

relm4::new_action_group!(pub AppActionGroup, "app");
relm4::new_stateful_action!(pub Copy, AppActionGroup, "copy-text", String, ());

relm4::new_action_group!(pub MainMenuActionGroup, "main");
relm4::new_stateless_action!(pub EditProfile, MainMenuActionGroup, "profile");

pub fn make_main_menu_actions(sender: AsyncComponentSender<Main>) -> SimpleActionGroup {
    let mut group = RelmActionGroup::<MainMenuActionGroup>::new();

    group.add_action(profile_action(sender));
    group.into_action_group()
}

fn profile_action(sender: AsyncComponentSender<Main>) -> RelmAction<EditProfile> {
    RelmAction::new_stateless(move |_| sender.input(MainInput::EditProfile))
}
