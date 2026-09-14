// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    fl,
    heading::{normalize_heading, shortest_delta, smooth_heading},
    rose::CompassRose,
    sensor::{self, SensorEvent},
};
use cosmic::{
    Application, Core, Element, Theme,
    app::context_drawer,
    executor,
    iced::{Length, Subscription},
    surface, theme,
    widget::{
        self,
        about::About,
        icon,
        menu::{self, ItemHeight, ItemWidth, key_bind::KeyBind},
    },
};
use std::{collections::HashMap, sync::LazyLock, time::Duration};

static MENU_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("responsive-menu"));

const REPOSITORY_URL: &str = "https://github.com/cosmic-utils/cosmic-ext-compass";
const SUPPORT_URL: &str = "https://github.com/cosmic-utils/cosmic-ext-compass/issues";
const WEBSITE_URL: &str = "https://cosmic-utils.org";
const APP_ICON: &[u8] =
    include_bytes!("../resources/icons/hicolor/256x256/apps/io.github.cosmic_utils.compass.png");

struct AboutDetails {
    name: String,
    comments: String,
    website_label: String,
    website_url: &'static str,
    repository_label: String,
    repository_url: &'static str,
    support_label: String,
    support_url: &'static str,
}

fn about_details() -> AboutDetails {
    AboutDetails {
        name: fl!("compass"),
        comments: fl!("desktop-comment"),
        website_label: fl!("website"),
        website_url: WEBSITE_URL,
        repository_label: fl!("repository"),
        repository_url: REPOSITORY_URL,
        support_label: fl!("support"),
        support_url: SUPPORT_URL,
    }
}

fn about_widget() -> About {
    let details = about_details();
    About::default()
        .name(details.name)
        .icon(icon::from_raster_bytes(APP_ICON))
        .version(env!("GIT_VERSION"))
        .author("Frederic Laing")
        .comments(details.comments)
        .license("GPL-3.0-only")
        .developers([("Frederic Laing", "frederic.laing.development@gmail.com")])
        .links([
            (details.website_label, details.website_url),
            (details.repository_label, details.repository_url),
            (details.support_label, details.support_url),
        ])
}

fn demo_motion_heading(elapsed_seconds: f32) -> f32 {
    let primary_turn = 125.0 * (elapsed_seconds * 0.32).sin();
    let hand_motion = 45.0 * (elapsed_seconds * 0.73 + 0.8).sin();
    (165.0 + primary_turn + hand_motion).rem_euclid(360.0)
}

#[derive(Clone, Copy, Debug)]
pub struct Flags {
    pub demo_heading: Option<f32>,
    pub demo_motion: bool,
}

#[derive(Clone, Debug)]
pub enum Message {
    OpenUrl(String),
    Sensor(SensorEvent),
    Surface(surface::Action<Message>),
    Tick,
    ToggleAbout,
    ToggleSettings,
    SetAppTheme(usize),
    ResetAllSettings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuItemAction {
    Settings,
    About,
}

impl menu::action::MenuAction for MenuItemAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Self::Settings => Message::ToggleSettings,
            Self::About => Message::ToggleAbout,
        }
    }
}

fn view_menu() -> (String, Vec<menu::Item<MenuItemAction, String>>) {
    (
        fl!("view"),
        vec![
            menu::Item::Button(fl!("menu-settings"), None, MenuItemAction::Settings),
            menu::Item::Button(fl!("menu-about"), None, MenuItemAction::About),
        ],
    )
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AppTheme {
    #[default]
    System,
    Dark,
    Light,
}

impl AppTheme {
    fn from_dropdown_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::System),
            1 => Some(Self::Dark),
            2 => Some(Self::Light),
            _ => None,
        }
    }

    fn dropdown_index(self) -> usize {
        match self {
            Self::System => 0,
            Self::Dark => 1,
            Self::Light => 2,
        }
    }

    fn theme(self) -> Theme {
        match self {
            Self::System => theme::system_preference(),
            Self::Dark => Theme::dark(),
            Self::Light => Theme::light(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContextPage {
    About,
    Settings,
}

fn open_url_message(url: &str) -> Message {
    Message::OpenUrl(url.to_owned())
}

fn forward_surface(action: surface::Action<Message>) -> cosmic::Action<Message> {
    cosmic::Action::Surface(action)
}

#[derive(Clone, Debug)]
enum SensorState {
    Connecting,
    Ready,
    Unavailable,
    AccessDenied,
    Failed,
    Demo,
}

impl SensorState {
    fn shows_compass(&self) -> bool {
        matches!(self, Self::Ready | Self::Demo)
    }
}

pub struct CompassApp {
    core: Core,
    about: About,
    menu_key_binds: HashMap<KeyBind, MenuItemAction>,
    context_page: ContextPage,
    app_theme: AppTheme,
    state: SensorState,
    target: Option<f32>,
    displayed: Option<f32>,
    demo: bool,
    demo_motion: bool,
    demo_elapsed: Duration,
}

impl CompassApp {
    #[must_use]
    pub fn app_theme(&self) -> AppTheme {
        self.app_theme
    }

    fn toggle_context_page(&mut self, page: ContextPage) {
        if self.core.window.show_context && self.context_page == page {
            self.core.window.show_context = false;
        } else {
            self.context_page = page;
            self.core.window.show_context = true;
        }
    }

    fn settings_drawer(&self) -> context_drawer::ContextDrawer<'_, Message> {
        let theme_options = vec![fl!("match-desktop"), fl!("dark"), fl!("light")];
        let appearance = widget::settings::section()
            .title(fl!("settings-appearance"))
            .add(
                widget::settings::item::builder(fl!("settings-theme")).control(widget::dropdown(
                    theme_options,
                    Some(self.app_theme.dropdown_index()),
                    Message::SetAppTheme,
                )),
            );
        let reset =
            widget::button::standard(fl!("settings-reset-all")).on_press(Message::ResetAllSettings);
        let content: Element<'_, Message> =
            widget::settings::view_column(vec![appearance.into(), reset.into()]).into();

        context_drawer::context_drawer(content, Message::ToggleSettings)
            .title(fl!("settings-title"))
    }
}

impl Application for CompassApp {
    type Executor = executor::Default;
    type Flags = Flags;
    type Message = Message;

    const APP_ID: &'static str = "io.github.cosmic_utils.compass";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Self::Flags) -> (Self, cosmic::app::Task<Self::Message>) {
        core.window.header_title = fl!("compass");
        core.window.show_headerbar = true;
        core.window.content_container = true;
        core.window.use_template = true;
        let about = about_widget();
        let demo = flags.demo_motion || flags.demo_heading.is_some();
        let initial_heading = flags
            .demo_heading
            .or_else(|| flags.demo_motion.then(|| demo_motion_heading(0.0)));
        (
            Self {
                core,
                about,
                menu_key_binds: HashMap::new(),
                context_page: ContextPage::About,
                app_theme: AppTheme::default(),
                state: if demo {
                    SensorState::Demo
                } else {
                    SensorState::Connecting
                },
                target: initial_heading,
                displayed: initial_heading,
                demo,
                demo_motion: flags.demo_motion,
                demo_elapsed: Duration::ZERO,
            },
            cosmic::app::Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if self.demo_motion {
            cosmic::iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
        } else if self.demo {
            Subscription::none()
        } else {
            let sensor = sensor::subscription().map(Message::Sensor);
            let needs_animation = match (self.displayed, self.target) {
                (Some(current), Some(target)) => shortest_delta(current, target).abs() > 0.05,
                (None, Some(_)) => true,
                _ => false,
            };

            if needs_animation {
                Subscription::batch([
                    sensor,
                    cosmic::iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick),
                ])
            } else {
                sensor
            }
        }
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        match message {
            Message::OpenUrl(url) => {
                if let Err(error) = open::that_detached(&url) {
                    tracing::warn!(%error, %url, "failed to open URL");
                }
            }
            Message::Surface(action) => {
                return cosmic::task::message(forward_surface(action));
            }
            Message::ToggleAbout => {
                self.toggle_context_page(ContextPage::About);
            }
            Message::ToggleSettings => {
                self.toggle_context_page(ContextPage::Settings);
            }
            Message::SetAppTheme(index) => {
                let Some(app_theme) = AppTheme::from_dropdown_index(index) else {
                    return cosmic::app::Task::none();
                };
                self.app_theme = app_theme;
                return cosmic::command::set_theme(app_theme.theme());
            }
            Message::ResetAllSettings => {
                self.app_theme = AppTheme::default();
                return cosmic::command::set_theme(self.app_theme.theme());
            }
            Message::Tick => {
                if self.demo_motion {
                    self.demo_elapsed += Duration::from_millis(16);
                    let heading = demo_motion_heading(self.demo_elapsed.as_secs_f32());
                    self.target = Some(heading);
                    self.displayed = Some(heading);
                } else if let Some(target) = self.target {
                    self.displayed = Some(self.displayed.map_or(target, |current| {
                        if shortest_delta(current, target).abs() <= 0.1 {
                            target
                        } else {
                            smooth_heading(current, target, 0.18)
                        }
                    }));
                }
            }
            Message::Sensor(event) => match event {
                SensorEvent::Heading(raw) => {
                    self.target = normalize_heading(raw);
                    if self.target.is_some() {
                        self.state = SensorState::Ready;
                    } else {
                        self.displayed = None;
                        self.state = SensorState::Unavailable;
                    }
                }
                SensorEvent::Unavailable => {
                    self.target = None;
                    self.displayed = None;
                    self.state = SensorState::Unavailable;
                }
                SensorEvent::AccessDenied => {
                    self.target = None;
                    self.displayed = None;
                    self.state = SensorState::AccessDenied;
                }
                SensorEvent::Failed(error) => {
                    tracing::warn!(%error, "sensor subscription stopped");
                    self.target = None;
                    self.displayed = None;
                    self.state = SensorState::Failed;
                }
            },
        }
        cosmic::app::Task::none()
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => {
                context_drawer::about(&self.about, open_url_message, Message::ToggleAbout)
            }
            ContextPage::Settings => self.settings_drawer(),
        })
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        vec![
            widget::responsive_menu_bar()
                .item_height(ItemHeight::Dynamic(40))
                .item_width(ItemWidth::Uniform(320))
                .spacing(4.0)
                .into_element(
                    &self.core,
                    &self.menu_key_binds,
                    MENU_ID.clone(),
                    Message::Surface,
                    vec![view_menu()],
                ),
        ]
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let status = match self.state {
            SensorState::Connecting => fl!("status-connecting"),
            SensorState::Ready => fl!("status-magnetic-north"),
            SensorState::Unavailable => fl!("status-unavailable"),
            SensorState::AccessDenied => fl!("status-access-denied"),
            SensorState::Failed => fl!("status-error"),
            SensorState::Demo => fl!("status-demo"),
        };

        if self.state.shows_compass() {
            let rose = widget::canvas(CompassRose {
                heading: self.displayed,
                status,
            })
            .width(Length::Fill)
            .height(Length::Fill);

            widget::container(rose)
                .padding([4, 0, 8, 0])
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            widget::container(widget::text::body(status))
                .padding(16)
                .center(Length::Fill)
                .into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::widget::menu::action::MenuAction as _;

    #[test]
    fn animated_demo_heading_moves_smoothly_in_both_directions() {
        let headings: Vec<_> = (0..=2_000)
            .map(|step| demo_motion_heading(step as f32 * 0.016))
            .collect();

        assert!(
            headings
                .iter()
                .all(|heading| heading.is_finite() && (0.0..360.0).contains(heading))
        );

        let deltas: Vec<_> = headings
            .windows(2)
            .map(|pair| shortest_delta(pair[0], pair[1]))
            .collect();
        assert!(deltas.iter().all(|delta| delta.abs() < 2.0));
        assert!(deltas.iter().any(|delta| *delta > 0.05));
        assert!(deltas.iter().any(|delta| *delta < -0.05));
    }

    #[test]
    fn compass_visibility_requires_a_reachable_sensor() {
        assert!(!SensorState::Connecting.shows_compass());
        assert!(!SensorState::Unavailable.shows_compass());
        assert!(!SensorState::AccessDenied.shows_compass());
        assert!(!SensorState::Failed.shows_compass());
        assert!(SensorState::Ready.shows_compass());
        assert!(SensorState::Demo.shows_compass());
    }

    #[test]
    fn view_menu_contains_localized_settings_and_about_actions() {
        crate::i18n::init(&[]);
        let (label, items) = view_menu();

        assert_eq!(label, "View");
        match items.as_slice() {
            [
                menu::Item::Button(settings_label, None, settings_action),
                menu::Item::Button(about_label, None, about_action),
            ] => {
                assert_eq!(settings_label, "Settings…");
                assert!(matches!(settings_action.message(), Message::ToggleSettings));
                assert_eq!(about_label, "About Compass…");
                assert!(matches!(about_action.message(), Message::ToggleAbout));
            }
            _ => panic!("View menu must contain enabled Settings and About actions"),
        }
    }

    #[test]
    fn embedded_about_icon_is_a_real_png() {
        assert!(APP_ICON.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(APP_ICON.len() > 1_024);
    }

    #[test]
    fn about_details_supply_localized_labels_and_project_links() {
        crate::i18n::init(&[]);
        let details = about_details();

        assert_eq!(details.name, "Compass");
        assert_eq!(details.comments, fl!("desktop-comment"));
        assert_eq!(details.website_label, "Website");
        assert_eq!(details.website_url, WEBSITE_URL);
        assert_eq!(details.repository_label, "Repository");
        assert_eq!(details.repository_url, REPOSITORY_URL);
        assert_eq!(details.support_label, "Support");
        assert_eq!(details.support_url, SUPPORT_URL);
    }

    #[test]
    fn about_url_callback_preserves_the_url() {
        assert!(matches!(
            open_url_message(REPOSITORY_URL),
            Message::OpenUrl(url) if url == REPOSITORY_URL
        ));
    }

    #[test]
    fn surface_actions_are_forwarded_to_libcosmic() {
        assert!(matches!(
            forward_surface(surface::Action::Ignore),
            cosmic::Action::Surface(surface::Action::Ignore)
        ));
    }
}
