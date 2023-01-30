use gtk::gdk;
use gtk::gio::SimpleActionGroup;
use gtk::prelude::DisplayExt;
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::AsyncComponentSender;

use crate::win::{Msg, Win};

relm4::new_action_group!(pub AuthorActionGroup, "author");
relm4::new_stateful_action!(pub Copy, AuthorActionGroup, "copy-hex", String, ());

/// Creates a GTK action group for author-related actions.
pub fn make_author_actions() -> SimpleActionGroup {
    let group = RelmActionGroup::<AuthorActionGroup>::new();

    group.add_action(&copy_action());
    group.into_action_group()
}

/// Copies a textual value into clipboard.
fn copy_action() -> RelmAction<Copy> {
    RelmAction::new_with_target_value(|_, string: String| {
        let display = gdk::Display::default().unwrap();
        let clipboard = display.clipboard();
        clipboard.set_text(&string);
    })
}

relm4::new_action_group!(pub MainMenuActionGroup, "main");
relm4::new_stateless_action!(pub EditProfile, MainMenuActionGroup, "profile");

pub fn make_main_menu_actions(sender: AsyncComponentSender<Win>) -> SimpleActionGroup {
    let group = RelmActionGroup::<MainMenuActionGroup>::new();

    group.add_action(&profile_action(sender));
    group.into_action_group()
}

fn profile_action(sender: AsyncComponentSender<Win>) -> RelmAction<EditProfile> {
    RelmAction::new_stateless(move |_| sender.input(Msg::EditProfile))
}
