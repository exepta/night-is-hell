pub mod player;
pub mod character;

use std::fs;
use std::path::Path;
use bevy::prelude::*;
use serde::Deserialize;
use crate::constants::ENTITY_JSON_PATH;

pub struct EntitiesModule;

impl Plugin for EntitiesModule {

    #[coverage(off)]
    fn build(&self, _app: &mut App) {
    }
}

#[derive(Debug, Clone, Reflect, Deserialize, PartialEq)]
pub struct EntityBaseInformation {
    pub localized: String,
    pub name: String,
    pub model_path: String,
    pub animations: Vec<EntityAnimation>,
    #[serde(default, rename = "type")]
    pub _type: EntityDataType
}

#[coverage(off)]
impl EntityBaseInformation {
    pub fn fetch(localized_name: &str) -> Result<Self, String>
    where Self: Sized + for<'de> Deserialize<'de>
    {
        let mut localized_parts = localized_name.split("::");
        let folder = localized_parts.next().ok_or("No folder found")?;
        let name = localized_parts.next().ok_or("No name found")?;

        if localized_parts.next().is_some() {
            return Err(format!("Too many segments in '{localized_name}'. Expected 'folder::name'"));
        }

        let file_name = if name.ends_with(".json") {
            name.to_string()
        } else {
            format!("{}.json", name)
        };

        let path = Path::new(ENTITY_JSON_PATH)
            .join(folder)
            .join(file_name);

        let file_content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

        let mut entity: Self = serde_json::from_str(&file_content)
            .map_err(|e| format!("Failed to parse JSON in {:?}: {}", path, e))?;

        entity._type = match folder {
            "characters" => EntityDataType::Character,
            "enemies" => EntityDataType::Enemy,
            "npc" => EntityDataType::Npc,
            _ => EntityDataType::Unknown,
        };

        Ok(entity)
    }

    #[coverage(off)]
    pub fn fetch_all() -> Result<Vec<Self>, String>
    where Self: Sized + for<'de> Deserialize<'de>
    {
        let mut results = Vec::new();

        let folders = fs::read_dir(ENTITY_JSON_PATH)
            .map_err(|e| format!("Failed to read directory '{}': {}", ENTITY_JSON_PATH, e))?;

        for entry in folders {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let folder_path = entry.path();

            if !folder_path.is_dir() {
                continue;
            }

            let folder_name = folder_path
                .file_name()
                .and_then(|folder_n| folder_n.to_str())
                .ok_or_else(|| format!("Failed to get folder name for {:?}", folder_path))?;

            let json_files = fs::read_dir(&folder_path)
                .map_err(|e| format!("Failed to read folder '{}': {}", folder_path.display(), e))?;

            for json_entry in json_files {
                let json_entry = json_entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                let json_path = json_entry.path();

                if json_path.is_file() && json_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                    let stem = json_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or_else(|| format!("Failed to get file stem for {:?}", json_path))?;

                    let localized_name = format!("{}::{}", folder_name, stem);

                    let entity = Self::fetch(&localized_name)
                        .map_err(|e| format!("Failed to fetch {}: {}", localized_name, e))?;

                    results.push(entity);
                }
            }
        }

        Ok(results)
    }

    /// Retrieves an animation by its name.
    ///
    /// # Arguments
    /// - `name` - The key of the animation.
    ///
    /// # Returns
    /// - `Some(&CharacterAnimation)` if found.
    /// - `None` if no matching animation exists.
    #[coverage(off)]
    pub fn get_animation_by_name(&self, name: &str) -> Option<&EntityAnimation> {
        self.animations.iter().find(|anim| anim.key == name)
    }
}

#[derive(Deserialize, Reflect, Debug, Clone, PartialEq)]
pub struct EntityAnimation {
    pub key: String,
    pub index: u16,
}

#[derive(Deserialize, Reflect, Debug, Default, Clone, PartialEq)]
pub enum EntityDataType {
    Character,
    Npc,
    Enemy,
    #[default]
    Unknown,
}

impl EntityDataType {

    #[coverage(off)]
    pub fn path(&self) -> &'static str {
        match self {
            EntityDataType::Character => "/characters",
            EntityDataType::Npc => "/npc",
            EntityDataType::Enemy => "/enemies",
            EntityDataType::Unknown => "/unknown",
        }
    }
}