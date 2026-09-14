// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    fl,
    geocode::{self, CacheKey, GeocodeEvent},
    heading::{normalize_heading, shortest_delta, smooth_heading},
    location::{self, LocationEvent, LocationFix, format_coordinates},
    rose::CompassRose,
    sensor::{self, SensorEvent, TiltEvent, TiltReading},
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
use i18n_embed::DesktopLanguageRequester;
use std::{collections::HashMap, sync::LazyLock, time::Duration};

static MENU_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("responsive-menu"));

const REPOSITORY_URL: &str = "https://github.com/cosmic-utils/cosmic-ext-compass";
const SUPPORT_URL: &str = "https://github.com/cosmic-utils/cosmic-ext-compass/issues";
const WEBSITE_URL: &str = "https://cosmic-utils.org";
const APP_ICON: &[u8] =
    include_bytes!("../resources/icons/hicolor/256x256/apps/org.cosmic_utils.compass.png");

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
            (
                fl!("osm-data-credit"),
                "https://www.openstreetmap.org/copyright",
            ),
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
    pub initial_theme: Option<AppTheme>,
}

#[derive(Clone, Debug)]
pub enum Message {
    Geocode(GeocodeEvent),
    Location(LocationEvent),
    OpenUrl(String),
    Sensor(SensorEvent),
    Tilt(TiltEvent),
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
    fn compass_enabled(&self) -> bool {
        matches!(self, Self::Ready | Self::Demo)
    }

    fn readout_status(&self) -> Option<String> {
        match self {
            Self::Connecting => Some(fl!("status-connecting")),
            Self::Ready => Some(fl!("status-magnetic-north")),
            Self::Unavailable => Some(fl!("status-unavailable")),
            Self::AccessDenied => Some(fl!("status-access-denied")),
            Self::Failed => Some(fl!("status-error")),
            Self::Demo => None,
        }
    }
}

fn requests_live_location(demo: bool) -> bool {
    !demo
}

fn current_locale() -> String {
    geocode::locale_preference(&DesktopLanguageRequester::requested_languages())
}

#[derive(Clone, Debug)]
enum PlaceState {
    Loading,
    OpenStreetMap(String),
    Unavailable,
}

#[derive(Clone, Debug)]
enum LocationState {
    Searching,
    Ready { fix: LocationFix, place: PlaceState },
    Unavailable,
    AccessDenied,
    Failed,
}

fn location_lines(state: &LocationState) -> Vec<String> {
    match state {
        LocationState::Searching => vec![fl!("location-searching")],
        LocationState::Ready { fix, place } => {
            let mut lines = vec![format_coordinates(fix.latitude, fix.longitude)];
            match place {
                PlaceState::OpenStreetMap(place_name) => lines.push(place_name.clone()),
                PlaceState::Loading => lines.push(fl!("location-place-searching")),
                PlaceState::Unavailable => lines.push(fl!("location-place-unavailable")),
            }
            lines.push(fl!(
                "location-accuracy",
                meters = (fix.accuracy_m.round() as i64)
            ));
            if let Some(altitude) = fix.altitude_m {
                lines.push(fl!(
                    "location-elevation",
                    meters = (altitude.round() as i64)
                ));
            } else {
                lines.push(fl!("location-elevation-unavailable"));
            }
            lines
        }
        LocationState::Unavailable => vec![fl!("location-unavailable")],
        LocationState::AccessDenied => vec![fl!("location-access-denied")],
        LocationState::Failed => vec![fl!("location-error")],
    }
}

fn osm_attribution_visible(state: &LocationState) -> bool {
    matches!(
        state,
        LocationState::Ready {
            place: PlaceState::OpenStreetMap(_),
            ..
        }
    )
}

fn apply_location_fix(state: &mut LocationState, mut fix: LocationFix, locale: &str) {
    fix.place_name = None;
    let next_key = CacheKey::for_display(fix.latitude, fix.longitude, locale);
    let place = match state {
        LocationState::Ready {
            fix: previous,
            place,
        } if CacheKey::for_display(previous.latitude, previous.longitude, locale) == next_key => {
            std::mem::replace(place, PlaceState::Loading)
        }
        _ => PlaceState::Loading,
    };
    *state = LocationState::Ready { fix, place };
}

fn apply_geocode_event(state: &mut LocationState, event: GeocodeEvent, locale: &str) {
    let LocationState::Ready { fix, place } = state else {
        return;
    };
    let expected = CacheKey::for_display(fix.latitude, fix.longitude, locale);
    match event {
        GeocodeEvent::Resolved { key, place_name } if key == expected => {
            *place = place_name.map_or(PlaceState::Unavailable, PlaceState::OpenStreetMap);
        }
        GeocodeEvent::Failed { key, kind } if key == expected => {
            tracing::warn!(?kind, "reverse geocoding failed");
            *place = PlaceState::Unavailable;
        }
        GeocodeEvent::Resolved { .. } | GeocodeEvent::Failed { .. } => {}
    }
}

pub struct CompassApp {
    core: Core,
    about: About,
    menu_key_binds: HashMap<KeyBind, MenuItemAction>,
    context_page: ContextPage,
    app_theme: AppTheme,
    state: SensorState,
    location_state: LocationState,
    tilt: Option<TiltReading>,
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

    const APP_ID: &'static str = "org.cosmic_utils.compass";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Self::Flags) -> (Self, cosmic::app::Task<Self::Message>) {
        core.window.header_title.clear();
        core.window.show_headerbar = true;
        core.window.content_container = true;
        core.window.use_template = true;
        let about = about_widget();
        let app_theme = flags.initial_theme.unwrap_or_default();
        let theme_task = if flags.initial_theme.is_some() {
            cosmic::command::set_theme(app_theme.theme())
        } else {
            cosmic::app::Task::none()
        };
        let demo = flags.demo_motion || flags.demo_heading.is_some();
        let initial_heading = flags
            .demo_heading
            .or_else(|| flags.demo_motion.then(|| demo_motion_heading(0.0)));
        let mut demo_fix = location::demo_location();
        let demo_place = demo_fix
            .place_name
            .take()
            .expect("demonstration location must have a place name");
        (
            Self {
                core,
                about,
                menu_key_binds: HashMap::new(),
                context_page: ContextPage::About,
                app_theme,
                state: if demo {
                    SensorState::Demo
                } else {
                    SensorState::Connecting
                },
                location_state: if demo {
                    LocationState::Ready {
                        fix: demo_fix,
                        place: PlaceState::OpenStreetMap(demo_place),
                    }
                } else {
                    LocationState::Searching
                },
                tilt: None,
                target: initial_heading,
                displayed: initial_heading,
                demo,
                demo_motion: flags.demo_motion,
                demo_elapsed: Duration::ZERO,
            },
            theme_task,
        )
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = Vec::new();

        if requests_live_location(self.demo) {
            subscriptions.push(location::subscription().map(Message::Location));
            if let LocationState::Ready {
                fix,
                place: PlaceState::Loading,
            } = &self.location_state
            {
                subscriptions
                    .push(geocode::subscription(fix, current_locale()).map(Message::Geocode));
            }
        }

        if self.demo_motion {
            subscriptions
                .push(cosmic::iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick));
        } else if !self.demo {
            subscriptions.push(sensor::subscription().map(Message::Sensor));
            subscriptions.push(sensor::tilt_subscription().map(Message::Tilt));
            let needs_animation = match (self.displayed, self.target) {
                (Some(current), Some(target)) => shortest_delta(current, target).abs() > 0.05,
                (None, Some(_)) => true,
                _ => false,
            };
            if needs_animation {
                subscriptions.push(
                    cosmic::iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick),
                );
            }
        }

        Subscription::batch(subscriptions)
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        match message {
            Message::Geocode(event) => {
                apply_geocode_event(&mut self.location_state, event, &current_locale());
            }
            Message::Location(event) => match event {
                LocationEvent::Fix(fix) => {
                    apply_location_fix(&mut self.location_state, fix, &current_locale());
                }
                LocationEvent::Unavailable => self.location_state = LocationState::Unavailable,
                LocationEvent::AccessDenied => self.location_state = LocationState::AccessDenied,
                LocationEvent::Failed(error) => {
                    tracing::warn!(%error, "location subscription stopped");
                    self.location_state = LocationState::Failed;
                }
            },

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
            Message::Tilt(event) => match event {
                TiltEvent::Reading(reading) => self.tilt = Some(reading),
                TiltEvent::Unavailable | TiltEvent::AccessDenied => self.tilt = None,
                TiltEvent::Failed(error) => {
                    tracing::warn!(%error, "accelerometer subscription stopped");
                    self.tilt = None;
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
        let status = self.state.readout_status();
        let compass_enabled = self.state.compass_enabled();
        let rose = widget::canvas(CompassRose {
            heading: self.displayed,
            tilt: self.tilt,
            compass_enabled,
            heading_warning: (!compass_enabled).then(|| status.clone()).flatten(),
            status: if compass_enabled {
                status.unwrap_or_default()
            } else {
                String::new()
            },
            location_lines: location_lines(&self.location_state),
            attribution: osm_attribution_visible(&self.location_state)
                .then(|| fl!("osm-attribution")),
        })
        .width(Length::Fill)
        .height(Length::Fill);

        widget::container(rose)
            .padding([4, 0, 8, 0])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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
    fn compass_stays_visible_but_is_disabled_without_a_magnetometer() {
        assert!(!SensorState::Connecting.compass_enabled());
        assert!(!SensorState::Unavailable.compass_enabled());
        assert!(!SensorState::AccessDenied.compass_enabled());
        assert!(!SensorState::Failed.compass_enabled());
        assert!(SensorState::Ready.compass_enabled());
        assert!(SensorState::Demo.compass_enabled());
    }

    #[test]
    fn view_menu_contains_only_standard_actions() {
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
            _ => panic!("View menu must contain only Settings and About actions"),
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
    fn live_location_does_not_depend_on_magnetometer_availability() {
        assert!(!requests_live_location(true));
        assert!(requests_live_location(false));
    }

    #[test]
    fn regular_window_has_no_visible_header_title() {
        crate::i18n::init(&[]);
        let (app, _) = <CompassApp as Application>::init(
            Core::default(),
            Flags {
                demo_heading: None,
                demo_motion: false,
                initial_theme: None,
            },
        );

        assert!(app.core.window.header_title.is_empty());
    }

    #[test]
    fn demo_mode_does_not_add_a_demonstration_status_line() {
        assert_eq!(SensorState::Demo.readout_status(), None);
    }

    #[test]
    fn demo_modes_use_burg_eltz_without_requesting_live_location() {
        crate::i18n::init(&[]);
        let (app, _) = <CompassApp as Application>::init(
            Core::default(),
            Flags {
                demo_heading: Some(165.0),
                demo_motion: false,
                initial_theme: None,
            },
        );

        let lines = location_lines(&app.location_state);
        assert_eq!(lines[0], "50°12′18″ N 7°20′12″ E");
        assert_eq!(lines[1], "Burg Eltz, Rhineland-Palatinate");
        assert!(lines[2].contains("12") && lines[2].contains("Accuracy"));
        assert!(lines[3].ends_with(" m Elevation"));
        assert!(osm_attribution_visible(&app.location_state));
        assert!(app.demo);
    }

    #[test]
    fn location_readings_use_only_available_geoclue_values() {
        crate::i18n::init(&[]);
        let fix = crate::location::demo_location();

        let lines = location_lines(&LocationState::Ready {
            place: PlaceState::OpenStreetMap(
                fix.place_name
                    .clone()
                    .expect("demo place name should be available"),
            ),
            fix,
        });
        assert_eq!(lines[0], "50°12′18″ N 7°20′12″ E");
        assert_eq!(lines[1], "Burg Eltz, Rhineland-Palatinate");
        assert!(lines[2].contains("12") && lines[2].contains("Accuracy"));
        assert!(lines[3].contains("320"));
        assert!(lines[3].ends_with(" m Elevation"));
        assert_eq!(
            location_lines(&LocationState::Searching),
            vec!["Waiting for a location reading…"]
        );
    }

    #[test]
    fn repeated_fix_at_the_same_displayed_position_keeps_the_place_name() {
        let mut previous = crate::location::demo_location();
        previous.place_name = None;
        let mut state = LocationState::Ready {
            fix: previous,
            place: PlaceState::OpenStreetMap("Wierschem, Rhineland-Palatinate".to_owned()),
        };
        let mut next = crate::location::demo_location();
        next.latitude += 0.000_01;

        apply_location_fix(&mut state, next, "en");

        assert!(matches!(
            state,
            LocationState::Ready {
                place: PlaceState::OpenStreetMap(_),
                ..
            }
        ));
    }

    #[test]
    fn fix_at_a_new_displayed_position_starts_automatic_lookup() {
        let mut previous = crate::location::demo_location();
        previous.place_name = None;
        let mut state = LocationState::Ready {
            fix: previous,
            place: PlaceState::OpenStreetMap("Wierschem, Rhineland-Palatinate".to_owned()),
        };
        let mut next = crate::location::demo_location();
        next.latitude += 1.0 / 3_600.0;

        apply_location_fix(&mut state, next, "en");

        assert!(matches!(
            state,
            LocationState::Ready {
                place: PlaceState::Loading,
                ..
            }
        ));
    }

    #[test]
    fn location_reading_reports_automatic_place_lookup() {
        crate::i18n::init(&[]);
        let mut fix = crate::location::demo_location();
        fix.place_name = None;
        let lines = location_lines(&LocationState::Ready {
            fix,
            place: PlaceState::Loading,
        });

        assert_eq!(lines[1], "Finding place name…");
        assert!(lines[2].contains("12") && lines[2].contains("Accuracy"));
    }

    #[test]
    fn matching_reverse_geocode_result_updates_the_visible_place() {
        crate::i18n::init(&[]);
        let mut fix = crate::location::demo_location();
        fix.place_name = None;
        let key = crate::geocode::CacheKey::for_display(fix.latitude, fix.longitude, "en");
        let mut state = LocationState::Ready {
            fix,
            place: PlaceState::Loading,
        };

        apply_geocode_event(
            &mut state,
            crate::geocode::GeocodeEvent::Resolved {
                key,
                place_name: Some("Wierschem, Rhineland-Palatinate".to_owned()),
            },
            "en",
        );

        let lines = location_lines(&state);
        assert_eq!(lines[1], "Wierschem, Rhineland-Palatinate");
        assert!(osm_attribution_visible(&state));
    }

    #[test]
    fn reverse_geocode_failures_preserve_the_fix() {
        crate::i18n::init(&[]);
        let mut fix = crate::location::demo_location();
        fix.place_name = None;
        let key = crate::geocode::CacheKey::for_display(fix.latitude, fix.longitude, "en");
        let mut state = LocationState::Ready {
            fix,
            place: PlaceState::Loading,
        };

        apply_geocode_event(
            &mut state,
            crate::geocode::GeocodeEvent::Failed {
                key,
                kind: crate::geocode::GeocodeErrorKind::Network,
            },
            "en",
        );

        let lines = location_lines(&state);
        assert_eq!(lines[0], "50°12′18″ N 7°20′12″ E");
        assert_eq!(lines[1], "Place name unavailable");
        assert!(lines[2].contains("12") && lines[2].contains("Accuracy"));
    }

    #[test]
    fn openstreetmap_attribution_is_separate_from_centered_readings() {
        crate::i18n::init(&[]);
        let fix = crate::location::demo_location();
        let lines = location_lines(&LocationState::Ready {
            fix,
            place: PlaceState::OpenStreetMap("Wierschem, Rhineland-Palatinate".to_owned()),
        });

        assert_eq!(lines[1], "Wierschem, Rhineland-Palatinate");
        assert!(!lines.iter().any(|line| line.contains("OpenStreetMap")));
        assert!(osm_attribution_visible(&LocationState::Ready {
            fix: crate::location::demo_location(),
            place: PlaceState::OpenStreetMap("Wierschem, Rhineland-Palatinate".to_owned()),
        }));
        assert!(!osm_attribution_visible(&LocationState::Searching));
    }

    #[test]
    fn location_readings_report_missing_place_and_elevation() {
        crate::i18n::init(&[]);
        let fix = crate::location::LocationFix::try_new(
            50.205_055_4,
            7.336_597_1,
            24.0,
            f64::MIN,
            "WiFi",
        )
        .unwrap();

        let lines = location_lines(&LocationState::Ready {
            fix,
            place: PlaceState::Loading,
        });
        assert_eq!(lines[0], "50°12′18″ N 7°20′12″ E");
        assert_eq!(lines[1], "Finding place name…");
        assert!(lines[2].contains("24"));
        assert_eq!(lines[3], "No elevation reading");
    }

    #[test]
    fn tilt_reading_is_independent_of_compass_availability() {
        let (mut app, _) = <CompassApp as Application>::init(
            Core::default(),
            Flags {
                demo_heading: None,
                demo_motion: false,
                initial_theme: None,
            },
        );
        let reading = crate::sensor::TiltReading::from_proxy_values("left-up", "tilted-up")
            .expect("test tilt should be valid");

        let _ = app.update(Message::Tilt(crate::sensor::TiltEvent::Reading(reading)));
        let _ = app.update(Message::Sensor(SensorEvent::Unavailable));

        assert_eq!(app.tilt, Some(reading));
    }

    #[test]
    fn surface_actions_are_forwarded_to_libcosmic() {
        assert!(matches!(
            forward_surface(surface::Action::Ignore),
            cosmic::Action::Surface(surface::Action::Ignore)
        ));
    }
}
