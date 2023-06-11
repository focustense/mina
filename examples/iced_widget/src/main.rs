use crate::attention_button::AttentionButton;
use crate::expander::Expander;
use crate::transformable::{Transform, TransformOperation, TransformStatus, Transformable};
use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, text};
use iced::{
    executor, theme, Alignment, Application, Background, Color, Command, Element, Font, Length,
    Renderer, Settings, Theme, Vector,
};
use iced_native::widget::Id;
use once_cell::sync::Lazy;

mod attention_button;
mod expander;
mod transformable;
mod widget_helpers;

const FONT_AWESOME_SOLID: Font = Font::External {
    name: "FontAwesomeSolid",
    bytes: include_bytes!("Font Awesome 6 Free-Solid-900.otf"),
};

static MENU_TRANSFORM_ID: Lazy<Id> = Lazy::new(Id::unique);

/// Demonstrates the used of custom Iced widgets to perform animations using self-contained state,
/// i.e. without requiring an explicit frame subscription ("Tick message") in the app and without
/// necessarily needing to involve a lot of custom style sheets and monolithic view logic to perform
/// the animations.
///
/// The widgets in this example are _fairly_ simple, not involving any sophisticated path-based
/// rendering, just the sort of basic transformations and effects that are commonly done in CSS such
/// as motion along an axis. Refer to the individual widgets for details.
///
/// This example also shows the importance of animation blending. Try spamming clicks on the menu
/// toggle button and watch the sidebar, or moving the mouse quickly in and out of the toggle button
/// or any of the left tabs. There should be no jank or sudden jumps.
fn main() -> iced::Result {
    App::run(Settings {
        antialiasing: true,
        ..Default::default()
    })
}

#[derive(Clone, Debug)]
enum Message {
    Ignored,
    ToggleMenu,
}

struct App {
    is_menu_visible: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            is_menu_visible: true,
        }
    }

    pub fn toggle_menu(&mut self) {
        self.is_menu_visible = !self.is_menu_visible;
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let app = App::new();
        (app, Command::none())
    }

    fn title(&self) -> String {
        "Mina-Iced animated widgets demo".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Message::ToggleMenu = message {
            self.toggle_menu();
            let next_status = if self.is_menu_visible {
                TransformStatus::Off
            } else {
                TransformStatus::On
            };
            Command::widget(TransformOperation::new(
                MENU_TRANSFORM_ID.clone(),
                next_status,
            ))
        } else {
            Command::none()
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let menu_tab = |label, icon, color| {
            button(
                Expander::new(
                    row![
                        text(icon)
                            .font(FONT_AWESOME_SOLID)
                            .size(36)
                            .width(50)
                            .horizontal_alignment(Horizontal::Center),
                        text(label).size(36).height(36).width(Length::Shrink),
                    ]
                    .align_items(Alignment::Center)
                    .spacing(24),
                )
                .padding([16, 24, 16, 40])
                .collapsed_width(50.0),
            )
            .padding(0)
            .style(theme::Button::Custom(Box::new(MenuTabStyle::new(color))))
            .on_press(Message::Ignored)
        };
        let menu_pane = Transformable::new(
            column![
                menu_tab("Play", "\u{f11b}", Color::from_rgb(0.0, 0.44, 0.72)),
                menu_tab("Mods", "\u{f12e}", Color::from_rgb(0.18, 0.48, 0.16)),
                menu_tab("Friends", "\u{f004}", Color::from_rgb(0.46, 0.41, 0.08)),
                menu_tab("Achievements", "\u{f091}", Color::from_rgb(0.64, 0.32, 0.15)),
                menu_tab("Settings", "\u{f013}", Color::from_rgb(0.69, 0.26, 0.48)),
            ]
            .width(350)
            .height(Length::Fill)
            .padding([100, 0, 0, 0])
            .spacing(24)
            .align_items(Alignment::Start),
        )
        .id(MENU_TRANSFORM_ID.clone())
        .off_transform(Transform::new(-24.0, 0.0))
        .on_transform(Transform::new(-120.0, 0.0));
        let menu_button_text = if self.is_menu_visible { "Hide Menu" } else { "Show Menu" };
        let menu_button = AttentionButton::new(
            text(menu_button_text)
                .style(Color::WHITE)
                .size(32.0)
                .horizontal_alignment(Horizontal::Center),
        )
        .radius(100.0)
        .background_color(Color::from_rgb(0.25, 0.25, 1.0))
        .on_press(Message::ToggleMenu);
        let content = container(menu_button)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([0, 350, 0, 0])
            .center_x()
            .center_y();
        row![menu_pane, content].into()
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }
}

struct MenuTabStyle {
    background_color: Color,
}

impl MenuTabStyle {
    pub fn new(background_color: Color) -> Self {
        Self { background_color }
    }
}

impl button::StyleSheet for MenuTabStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(self.background_color)),
            border_radius: 16.0,
            shadow_offset: Vector::new(5.0, 5.0),
            text_color: Color::WHITE,
            ..Default::default()
        }
    }
}
