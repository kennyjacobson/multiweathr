use crate::animation_manager::AnimationManager;
use crate::app_state::AppState;
use crate::config::{Config, LocationDisplay, Provider};
use crate::error::WeatherError;
use crate::render::TerminalRenderer;
use crate::scene::city::CityScene;
use crate::scene::lake_house::LakeHouseScene;
use crate::scene::overlay::OverlayRegistry;
use crate::scene::ranch::RanchScene;
use crate::scene::world::WorldScene;
use crate::scene::{SceneContext, SceneRegistry};
use crate::theme::ThemeRegistry;

use crate::weather::provider::WeatherProvider;
use crate::weather::provider::met_office::{MetOfficeProvider, MetOfficeProviderConfig};
use crate::weather::types::CelestialEvents;
use crate::weather::{
    OpenMeteoProvider, WeatherClient, WeatherCondition, WeatherData, WeatherLocation,
};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use serde::Deserialize;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const REFRESH_INTERVAL: Duration = Duration::from_secs(300);
const INPUT_POLL_FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_millis(1000 / INPUT_POLL_FPS);
const DEFAULT_THEME_ID: &str = "default";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ThemeBindings {
    theme_id: &'static str,
    scene_id: &'static str,
    overlay_id: Option<&'static str>,
}

fn resolve_theme_bindings(
    themes: &ThemeRegistry,
    scenes: &SceneRegistry,
    overlays: &OverlayRegistry,
) -> ThemeBindings {
    let active_theme = themes.active();
    let mut theme_id = active_theme.id;
    let mut scene_id = active_theme.scene_id;
    let mut overlay_id = active_theme.overlay_id;

    let scene_missing = scenes.get(scene_id).is_none();
    if scene_missing {
        if theme_id != DEFAULT_THEME_ID {
            eprintln!(
                "Warning: theme '{}' references missing scene '{}'. Falling back to '{}'.",
                theme_id, scene_id, DEFAULT_THEME_ID
            );
            let fallback_theme = themes
                .get(DEFAULT_THEME_ID)
                .expect("default theme must be registered");
            theme_id = fallback_theme.id;
            scene_id = fallback_theme.scene_id;
            overlay_id = fallback_theme.overlay_id;
        } else {
            panic!("default theme references missing scene '{}'.", scene_id);
        }
    }

    if scenes.get(scene_id).is_none() {
        panic!(
            "theme '{}' references missing scene '{}', and no fallback scene is available",
            theme_id, scene_id
        );
    }

    let validated_overlay = overlay_id.and_then(|id| {
        if overlays.get(id).is_some() {
            Some(id)
        } else {
            eprintln!(
                "Warning: theme '{}' references missing overlay '{}'. Overlay disabled.",
                theme_id, id
            );
            None
        }
    });

    ThemeBindings {
        theme_id,
        scene_id,
        overlay_id: validated_overlay,
    }
}

/// Picks the scene a location asked for, or the theme's scene when it named none
/// or named one that isn't registered.
fn resolve_location_scene(
    requested: Option<&str>,
    fallback: &'static str,
    scenes: &SceneRegistry,
) -> &'static str {
    match requested {
        None => fallback,
        Some(id) => match scenes.get(id) {
            Some(scene) => scene.id(),
            None => {
                eprintln!(
                    "Warning: scene '{}' is not registered, falling back to '{}'.",
                    id, fallback
                );
                fallback
            }
        },
    }
}

fn generate_offline_weather(rng: &mut impl rand::Rng) -> WeatherData {
    use chrono::{Local, Timelike};
    use rand::RngExt;

    let now = Local::now();
    let hour = now.hour();
    let is_day = (6..18).contains(&hour);

    let conditions = [
        WeatherCondition::Clear,
        WeatherCondition::PartlyCloudy,
        WeatherCondition::Cloudy,
        WeatherCondition::Rain,
    ];

    let condition = conditions[rng.random_range(0..conditions.len())];

    WeatherData {
        condition,
        temperature: rng.random_range(10.0..25.0),
        precipitation: if condition.is_raining() {
            rng.random_range(1.0..5.0)
        } else {
            0.0
        },
        wind_speed: rng.random_range(5.0..15.0),
        wind_direction: rng.random_range(0.0..360.0),
        sun: CelestialEvents::from_bool(is_day),
        moon_phase: Some(0.5),
        timestamp: now.format("%Y-%m-%dT%H:%M:%S").to_string(),
        attribution: "".to_string(),
    }
}

/// A location the app shows, with the latest weather fetched for it.
struct LocationSlot {
    location: WeatherLocation,
    city_name: Option<String>,
    scene_id: &'static str,
    weather: Option<WeatherData>,
    error: Option<String>,
}

pub struct App {
    state: AppState,
    animations: AnimationManager,
    scenes: SceneRegistry,
    overlays: OverlayRegistry,
    themes: ThemeRegistry,
    active_overlay_id: Option<&'static str>,
    slots: Vec<LocationSlot>,
    current_slot: usize,
    rotate_interval: Duration,
    last_rotation: Instant,
    weather_receiver: mpsc::Receiver<(usize, Result<WeatherData, WeatherError>)>,
    hide_hud: bool,
}

impl App {
    pub fn new(
        config: &Config,
        simulate_condition: Option<String>,
        simulate_night: bool,
        show_leaves: bool,
        term_width: u16,
        term_height: u16,
        themes: ThemeRegistry,
    ) -> Self {
        let mut scenes = SceneRegistry::new();
        scenes.register(Box::new(WorldScene::new(term_width, term_height)));
        scenes.register(Box::new(RanchScene::new(term_width, term_height)));
        scenes.register(Box::new(LakeHouseScene::new(term_width, term_height)));
        scenes.register(Box::new(CityScene::new(term_width, term_height)));

        let overlays = OverlayRegistry::new();
        let bindings = resolve_theme_bindings(&themes, &scenes, &overlays);

        let mut slots: Vec<LocationSlot> = if config.is_rotating() {
            config
                .locations
                .iter()
                .map(|loc| LocationSlot {
                    location: WeatherLocation {
                        latitude: loc.latitude,
                        longitude: loc.longitude,
                        elevation: None,
                    },
                    city_name: loc.name.clone(),
                    scene_id: resolve_location_scene(
                        loc.scene.as_deref(),
                        bindings.scene_id,
                        &scenes,
                    ),
                    weather: None,
                    error: None,
                })
                .collect()
        } else {
            vec![LocationSlot {
                location: WeatherLocation {
                    latitude: config.location.latitude,
                    longitude: config.location.longitude,
                    elevation: None,
                },
                city_name: config.location.city.clone(),
                scene_id: bindings.scene_id,
                weather: None,
                error: None,
            }]
        };

        // Rotating locations are labelled by name, falling back to coordinates.
        let location_display = if config.is_rotating() {
            LocationDisplay::City
        } else {
            config.location.display
        };

        let state = AppState::new(
            slots[0].location,
            slots[0].city_name.clone(),
            location_display,
            config.location.hide,
            config.units,
        );
        let animations = AnimationManager::new(term_width, term_height, show_leaves);

        let (tx, rx) = mpsc::channel(slots.len());

        if let Some(ref condition_str) = simulate_condition {
            let simulated_condition =
                condition_str
                    .parse::<WeatherCondition>()
                    .unwrap_or_else(|e| {
                        eprintln!("{}", e);
                        WeatherCondition::Clear
                    });

            let weather = WeatherData {
                condition: simulated_condition,
                temperature: 20.0,
                precipitation: if simulated_condition.is_raining() {
                    2.5
                } else {
                    0.0
                },
                wind_speed: if simulated_condition.is_thunderstorm() {
                    45.0
                } else {
                    10.0
                },
                wind_direction: 225.0,
                sun: CelestialEvents::from_bool(!simulate_night),
                moon_phase: Some(0.5),
                timestamp: "simulated".to_string(),
                attribution: "".to_string(),
            };

            for slot in &mut slots {
                slot.weather = Some(weather.clone());
            }
        } else {
            let wanted_provider = config
                .provider
                .keys()
                .next()
                .cloned()
                .unwrap_or(Provider::default());

            let provider: Arc<dyn WeatherProvider> = match wanted_provider {
                Provider::OpenMeteo => Arc::new(OpenMeteoProvider::new()),
                Provider::MetOffice => {
                    let provider_config = {
                        if let Some(provider_config) = config.provider.get(&wanted_provider) {
                            MetOfficeProviderConfig::deserialize(provider_config.clone()).unwrap()
                        } else {
                            MetOfficeProviderConfig::default()
                        }
                    };
                    Arc::new(MetOfficeProvider::new(provider_config).unwrap())
                }
            };

            let units = config.units;

            // One fetch loop per location, so rotating never waits on the network.
            for (index, slot) in slots.iter().enumerate() {
                let weather_client = WeatherClient::new(provider.clone(), REFRESH_INTERVAL);
                let location = slot.location;
                let tx = tx.clone();

                tokio::spawn(async move {
                    loop {
                        let result = weather_client
                            .get_current_weather(&location, &units, wanted_provider)
                            .await;
                        if tx.send((index, result)).await.is_err() {
                            break;
                        }
                        tokio::time::sleep(REFRESH_INTERVAL).await;
                    }
                });
            }
        }

        let mut app = Self {
            state,
            animations,
            scenes,
            overlays,
            themes,
            active_overlay_id: bindings.overlay_id,
            slots,
            current_slot: 0,
            rotate_interval: Duration::from_secs(config.rotate_seconds()),
            last_rotation: Instant::now(),
            weather_receiver: rx,
            hide_hud: config.hide_hud,
        };

        if simulate_condition.is_some() {
            app.show_current_slot(&mut rand::rng());
        }

        app
    }

    fn apply_weather(&mut self, weather: WeatherData) {
        let rain_intensity = weather.condition.rain_intensity();
        let snow_intensity = weather.condition.snow_intensity();
        let fog_intensity = weather.condition.fog_intensity();
        let wind_speed = weather.wind_speed;
        let wind_direction = weather.wind_direction;

        if let Some(moon_phase) = weather.moon_phase {
            self.animations.update_moon_phase(moon_phase);
        }

        self.state.update_weather(weather);
        self.animations.update_rain_intensity(rain_intensity);
        self.animations.update_snow_intensity(snow_intensity);
        self.animations.update_fog_intensity(fog_intensity);
        self.animations
            .update_wind(wind_speed as f32, wind_direction as f32);
    }

    /// Shows the current slot's latest weather and returns the attribution text.
    fn show_current_slot(&mut self, rng: &mut impl rand::Rng) -> String {
        let slot = &self.slots[self.current_slot];
        let weather = slot.weather.clone();
        let error = slot.error.clone();

        match (weather, error) {
            (Some(weather), error) => {
                let attribution = weather.attribution.clone();
                self.apply_weather(weather);
                match error {
                    Some(error_msg) => {
                        self.state.set_offline_mode(true);
                        format!("Provider failed with {error_msg}")
                    }
                    None => attribution,
                }
            }
            (None, Some(error_msg)) => {
                self.apply_weather(generate_offline_weather(rng));
                self.state.set_offline_mode(true);
                format!("Provider failed with {error_msg} - Simulating")
            }
            (None, None) => "Awaiting weather data".to_string(),
        }
    }

    /// Moves `offset` slots forward (or back, when negative) and returns the
    /// attribution text for the newly shown location.
    fn switch_slot(&mut self, offset: isize, rng: &mut impl rand::Rng) -> String {
        let len = self.slots.len() as isize;
        self.current_slot = (self.current_slot as isize + offset).rem_euclid(len) as usize;
        self.last_rotation = Instant::now();

        let slot = &self.slots[self.current_slot];
        self.state
            .set_location(slot.location, slot.city_name.clone());
        self.show_current_slot(rng)
    }

    pub async fn run(&mut self, renderer: &mut TerminalRenderer) -> io::Result<()> {
        let mut rng = rand::rng();
        let mut attribution = "Awaiting weather data".to_string();

        loop {
            match self.weather_receiver.try_recv() {
                Ok((index, result)) => {
                    let slot = &mut self.slots[index];
                    match result {
                        Ok(weather) => {
                            slot.weather = Some(weather);
                            slot.error = None;
                        }
                        Err(error) => {
                            slot.error = Some(match &error {
                                WeatherError::Network(net_err) => net_err.user_friendly_message(),
                                _ => format!("Failed to fetch weather: {}", error),
                            });
                        }
                    }

                    if index == self.current_slot {
                        attribution = self.show_current_slot(&mut rng);
                    }
                }
                Err(e) => {
                    if e == mpsc::error::TryRecvError::Disconnected {
                        attribution = "".to_string();
                    }
                }
            }

            if self.slots.len() > 1 && self.last_rotation.elapsed() >= self.rotate_interval {
                attribution = self.switch_slot(1, &mut rng);
            }

            renderer.clear()?;

            let theme = self.themes.active();
            let palette = &theme.palette;

            let (term_width, term_height) = renderer.get_size();
            let scene = self
                .scenes
                .get_mut(self.slots[self.current_slot].scene_id)
                .expect("active scene must be registered");
            scene.update_size(term_width, term_height);

            let layout = scene.layout();
            let ctx = SceneContext {
                conditions: &self.state.weather_conditions,
                palette,
                wind_speed_ms: self
                    .state
                    .current_weather
                    .as_ref()
                    .map_or(0.0, |weather| weather.wind_speed),
            };

            self.animations.render_background(
                renderer,
                &self.state.weather_conditions,
                &self.state,
                &layout,
                &mut rng,
            )?;

            scene.render(renderer, &ctx)?;

            if let Some(ov_id) = self.active_overlay_id {
                if let Some(overlay) = self.overlays.get_mut(ov_id) {
                    overlay.update_size(term_width, term_height);
                    overlay.render(renderer, &ctx, &layout)?;
                }
            }

            self.animations.render_chimney_smoke(
                renderer,
                &self.state.weather_conditions,
                &self.state,
                &layout,
                &mut rng,
            )?;

            self.animations.render_foreground(
                renderer,
                &self.state.weather_conditions,
                &self.state,
                &layout,
                &mut rng,
            )?;

            self.state.update_loading_animation();
            self.state.update_cached_info();

            if !self.hide_hud {
                renderer.render_line_colored(
                    2,
                    1,
                    &self.state.cached_weather_info,
                    crossterm::style::Color::Cyan,
                )?;
            }

            let attribution_x = if term_width > attribution.len() as u16 {
                term_width - attribution.len() as u16 - 2
            } else {
                0
            };
            let attribution_y = if term_height > 0 { term_height - 1 } else { 0 };
            renderer.render_line_colored(
                attribution_x,
                attribution_y,
                &attribution,
                crossterm::style::Color::DarkGrey,
            )?;

            renderer.flush()?;

            if event::poll(FRAME_DURATION)? {
                match event::read()? {
                    Event::Resize(width, height) => {
                        renderer.manual_resize(width, height)?;
                        let (new_width, new_height) = renderer.get_size();
                        self.animations.on_resize(new_width, new_height);
                    }
                    Event::Key(key_event) => match key_event.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => break,
                        KeyCode::Char('c')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            break;
                        }
                        KeyCode::Char('n') | KeyCode::Right if self.slots.len() > 1 => {
                            attribution = self.switch_slot(1, &mut rng);
                        }
                        KeyCode::Char('p') | KeyCode::Left if self.slots.len() > 1 => {
                            attribution = self.switch_slot(-1, &mut rng);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::TerminalRenderer;
    use crate::scene::overlay::SceneOverlay;
    use crate::scene::{Scene, SceneContext, SceneLayout};
    use crate::theme::catalogue::DEFAULT_PALETTE;
    use crate::theme::{Theme, ThemeRegistry};
    use std::io;

    struct TestScene {
        id: &'static str,
    }

    impl TestScene {
        fn new(id: &'static str) -> Self {
            Self { id }
        }
    }

    impl Scene for TestScene {
        fn id(&self) -> &'static str {
            self.id
        }

        fn update_size(&mut self, _width: u16, _height: u16) {}

        fn render(
            &self,
            _renderer: &mut TerminalRenderer,
            _ctx: &SceneContext<'_>,
        ) -> io::Result<()> {
            Ok(())
        }

        fn layout(&self) -> SceneLayout {
            SceneLayout {
                ground_y: 0,
                chimney_pos: None,
                width: 0,
                height: 0,
            }
        }
    }

    struct TestOverlay {
        id: &'static str,
    }

    impl TestOverlay {
        fn new(id: &'static str) -> Self {
            Self { id }
        }
    }

    impl SceneOverlay for TestOverlay {
        fn id(&self) -> &'static str {
            self.id
        }

        fn update_size(&mut self, _width: u16, _height: u16) {}

        fn render(
            &self,
            _renderer: &mut TerminalRenderer,
            _ctx: &SceneContext<'_>,
            _layout: &SceneLayout,
        ) -> io::Result<()> {
            Ok(())
        }
    }

    fn scene_registry_with_world() -> SceneRegistry {
        let mut scenes = SceneRegistry::new();
        scenes.register(Box::new(TestScene::new("world")));
        scenes
    }

    #[test]
    fn bindings_fall_back_to_default_when_scene_missing() {
        let scenes = scene_registry_with_world();
        let overlays = OverlayRegistry::new();
        let mut themes = ThemeRegistry::new();
        themes.register(Theme {
            id: "custom",
            display_name: "Custom",
            scene_id: "unknown",
            overlay_id: None,
            palette: DEFAULT_PALETTE,
        });
        themes.set_active("custom").unwrap();

        let bindings = resolve_theme_bindings(&themes, &scenes, &overlays);

        assert_eq!(bindings.theme_id, DEFAULT_THEME_ID);
        assert_eq!(bindings.scene_id, "world");
        assert_eq!(bindings.overlay_id, None);
    }

    #[test]
    fn bindings_disable_unregistered_overlay() {
        let scenes = scene_registry_with_world();
        let overlays = OverlayRegistry::new();
        let mut themes = ThemeRegistry::new();
        themes.register(Theme {
            id: "overlay-theme",
            display_name: "Overlay Theme",
            scene_id: "world",
            overlay_id: Some("hud"),
            palette: DEFAULT_PALETTE,
        });
        themes.set_active("overlay-theme").unwrap();

        let bindings = resolve_theme_bindings(&themes, &scenes, &overlays);

        assert_eq!(bindings.theme_id, "overlay-theme");
        assert_eq!(bindings.scene_id, "world");
        assert_eq!(bindings.overlay_id, None);
    }

    #[test]
    fn bindings_keep_registered_overlay() {
        let scenes = scene_registry_with_world();
        let mut overlays = OverlayRegistry::new();
        overlays.register(Box::new(TestOverlay::new("hud")));
        let mut themes = ThemeRegistry::new();
        themes.register(Theme {
            id: "overlay",
            display_name: "Overlay",
            scene_id: "world",
            overlay_id: Some("hud"),
            palette: DEFAULT_PALETTE,
        });
        themes.set_active("overlay").unwrap();

        let bindings = resolve_theme_bindings(&themes, &scenes, &overlays);

        assert_eq!(bindings.theme_id, "overlay");
        assert_eq!(bindings.overlay_id, Some("hud"));
    }
}
