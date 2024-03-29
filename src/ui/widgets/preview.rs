use gtk;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{gdk, glib};
use relm4::{ComponentParts, ComponentSender, SimpleComponent};

use crate::app::action::CopyText;
use crate::nostr::preview;

#[derive(Debug)]
pub struct Preview {
    preview: preview::Preview,
}

#[relm4::component(pub)]
impl SimpleComponent for Preview {
    type Init = preview::Preview;
    type Input = ();
    type Output = ();

    #[rustfmt::skip]
    view! {
        #[name = "grid"]
        gtk::Grid {
            add_css_class: "preview",
            attach[1, 1, 1, 1]: infobox = &gtk::Box {
                add_css_class: "infobox",
                set_orientation: gtk::Orientation::Vertical,
                set_hexpand: true,
                set_cursor_from_name: Some("pointer"),
                gtk::Label {
                    set_label?: preview.title(),
                    add_css_class: "title",
                    set_xalign: 0.0,
                    set_wrap_mode: gtk::pango::WrapMode::Word,
                    set_wrap: true,
                    set_visible: preview.title().is_some()
                },
                gtk::Label {
                    set_label?: preview.description(),
                    add_css_class: "description",
                    set_xalign: 0.0,
                    set_wrap_mode: gtk::pango::WrapMode::Word,
                    set_wrap: true,
                    set_visible: preview.description().is_some()
                },
                gtk::Label {
                    set_label: preview.url().as_ref(),
                    add_css_class: "url",
                    set_ellipsize: gtk::pango::EllipsizeMode::Middle,
                    set_xalign: 0.0
                },
            },
        }
    }

    menu! {
        link_menu: {
            "Copy link address" => CopyText(preview.url().to_string())
        }
    }

    fn init(
        preview: Self::Init,
        root: &Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();

        if let Some(thumbnail) = preview.thumbnail() {
            let texture = thumbnail.texture();
            let picture = gtk::Picture::builder()
                .paintable(texture)
                .content_fit(gtk::ContentFit::Cover)
                .width_request(400)
                .height_request(
                    (f64::from(texture.height()) / f64::from(texture.width()) * 400.0).ceil()
                        as i32,
                )
                .build();

            widgets.grid.attach(&picture, 1, 0, 1, 1);

            picture.add_controller(context_menu(&picture, thumbnail));
            picture_action_group(texture).register_for_widget(picture);
        }

        let model = Preview { preview };

        let infobox_context_menu = gtk::GestureClick::builder().button(3).build();
        infobox_context_menu.connect_pressed(
            clone!(@weak widgets.infobox as infobox, @strong link_menu as menu => move |_, _, x, y| {
                let popover = gtk::PopoverMenu::builder()
                    .menu_model(&menu)
                    .has_arrow(false)
                    .pointing_to(&gdk::Rectangle::new(x as i32, y as i32, 1, 1))
                    .build();
                popover.set_parent(&infobox);
                popover.popup();
            }),
        );
        widgets.infobox.add_controller(infobox_context_menu);

        ComponentParts { model, widgets }
    }
}

/// Creates an event controller for opening popup menu on picture.
fn context_menu(picture: &gtk::Picture, thumbnail: &preview::Thumbnail) -> gtk::GestureClick {
    relm4::menu! {
        image_menu: {
            "Copy image" => CopyImage,
            "Copy image address" => CopyText(thumbnail.url().to_string()),
        }
    }

    let click = gtk::GestureClick::builder().button(3).build();
    click.connect_pressed(
        clone!(@weak picture, @strong image_menu as menu => move |_, _, x, y| {
            let popover = gtk::PopoverMenu::builder()
                .menu_model(&menu)
                .has_arrow(false)
                .pointing_to(&gdk::Rectangle::new(x as i32, y as i32, 1, 1))
                .build();
            popover.set_parent(&picture);
            popover.popup();
        }),
    );

    click
}

use relm4::actions::{RelmAction, RelmActionGroup};

relm4::new_action_group!(PictureActionGroup, "picture");
relm4::new_stateless_action!(CopyImage, PictureActionGroup, "copy-image");

/// Creates an action group 'picture' with all the actions available
/// on a single picture.
fn picture_action_group(texture: &gdk::Texture) -> RelmActionGroup<PictureActionGroup> {
    let mut group = RelmActionGroup::<PictureActionGroup>::new();

    group.add_action(RelmAction::<CopyImage>::new_stateless(
        clone!(@weak texture => move |_| {
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_texture(&texture);
            }
        }),
    ));

    group
}
