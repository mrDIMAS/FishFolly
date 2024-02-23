use fyrox::{core::log::Log, engine::GraphicsContext, renderer::QualitySettings, scene::Scene};
use ron::ser::to_string_pretty;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

pub struct Settings(SettingsData);

impl Settings {
    pub fn load() -> Self {
        Self(SettingsData::load())
    }

    pub fn read(&self) -> SettingsDataRef {
        SettingsDataRef(&self.0)
    }

    pub fn write(&mut self) -> SettingsDataRefMut {
        SettingsDataRefMut(&mut self.0)
    }
}

pub struct SettingsDataRef<'a>(&'a SettingsData);

impl<'a> Deref for SettingsDataRef<'a> {
    type Target = SettingsData;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub struct SettingsDataRefMut<'a>(&'a mut SettingsData);

impl<'a> Deref for SettingsDataRefMut<'a> {
    type Target = SettingsData;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for SettingsDataRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a> Drop for SettingsDataRefMut<'a> {
    fn drop(&mut self) {
        self.0.save();
    }
}

#[derive(Serialize, Deserialize)]
pub struct SettingsData {
    pub graphics_quality: usize,
    pub sound_volume: f32,
    pub music_volume: f32,
    pub graphics_presets: Vec<(String, QualitySettings)>,
    pub mouse_sensitivity: f32,
    pub mouse_smoothness: f32,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            graphics_quality: 3,
            sound_volume: 100.0,
            music_volume: 100.0,
            graphics_presets: vec![
                ("Low".to_string(), QualitySettings::low()),
                ("Medium".to_string(), QualitySettings::medium()),
                ("High".to_string(), QualitySettings::high()),
                ("Ultra".to_string(), QualitySettings::ultra()),
            ],
            mouse_sensitivity: 0.5,
            mouse_smoothness: 0.75,
        }
    }
}

impl SettingsData {
    pub fn save(&self) {
        match to_string_pretty(self, Default::default()) {
            Ok(serialized) => match File::create("game_settings.ron") {
                Ok(mut file) => {
                    Log::verify(file.write_all(serialized.as_bytes()));
                }
                Err(err) => Log::err(format!(
                    "Unable to write settings file on disk. Reason {:?}",
                    err
                )),
            },
            Err(err) => Log::err(format!(
                "Unable to serialize settings file. Reason {:?}",
                err
            )),
        }
    }

    pub fn load() -> Self {
        match File::open("game_settings.ron") {
            Ok(mut file) => {
                let mut file_content = String::new();
                match file.read_to_string(&mut file_content) {
                    Ok(_) => match ron::from_str(&file_content) {
                        Ok(settings) => {
                            return settings;
                        }
                        Err(err) => Log::err(format!(
                            "Unable to deserialize settings file. Reason {:?}",
                            err
                        )),
                    },
                    Err(err) => Log::err(format!(
                        "Unable to read settings file content. Reason {:?}",
                        err
                    )),
                }
            }
            Err(err) => Log::err(format!("Unable to read settings file. Reason {:?}", err)),
        }

        Log::err("Failed to load settings, fallback to defaults.");

        Default::default()
    }

    pub fn apply_sound_volume(&self, scene: &Scene) {
        scene
            .graph
            .sound_context
            .state()
            .bus_graph_mut()
            .primary_bus_mut()
            .set_gain((self.sound_volume / 100.0).clamp(0.0, 1.0));
    }

    pub fn apply_graphics_settings(&self, graphics_context: &mut GraphicsContext) {
        if let GraphicsContext::Initialized(graphics_context) = graphics_context {
            if let Some((_, settings)) = self.graphics_presets.get(self.graphics_quality) {
                Log::verify(graphics_context.renderer.set_quality_settings(settings));
            }
        }
    }
}
