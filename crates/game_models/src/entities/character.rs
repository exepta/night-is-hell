use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::entities::EntityBaseInformation;

/// The character. Is not in the world! Look at `Player` for more information.
#[derive(Component, Reflect, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Character {
    #[serde(skip)]
    pub id: Option<Entity>,
    #[serde(skip)]
    pub base_info: Option<EntityBaseInformation>,
    #[serde(default)]
    pub skill_attributes: CharacterSkillAttributes,
    #[serde(default)]
    pub damage_attributes: CharacterDamageAttributes,
    #[serde(default)]
    pub base_attributes: CharacterBaseAttributes,
    #[serde(default)]
    pub current_stats: CharacterCurrentStats,
    #[serde(default)]
    pub world_stats: CharacterWorldStats,
}

/// Contains the character's current in-game stats,
/// such as health, attack, and speed, which may change during gameplay.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterCurrentStats {
    pub hp: f64,
    pub ability_points: f64,
    pub super_armor: f64,
    pub attack: f64,
    pub defense: f64,
    pub speed: f64,
    pub crit_rate: f64,
    pub crit_damage: f64,
}

/// Represents the character's base stats before any modifications,
/// typically used as the starting point or baseline values.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterBaseAttributes {
    pub hp: f64,
    pub ability_points: f64,
    pub super_armor: f64,
    pub attack: f64,
    pub defense: f64,
    pub speed: f64,
    pub crit_rate: f64,
    pub crit_damage: f64,
}

/// Describes all elemental or magical damage types the character
/// can deal, including raw damage and "wds" modifiers for each type.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterDamageAttributes {
    pub fire_damage: f64,
    pub fire_wds: f64,
    pub lightning_damage: f64,
    pub lightning_wds: f64,
    pub water_damage: f64,
    pub water_wds: f64,
    pub ice_damage: f64,
    pub ice_wds: f64,
    pub nature_damage: f64,
    pub nature_wds: f64,
    pub physical_damage: f64,
    pub physical_wds: f64,
    pub demonic_damage: f64,
    pub demonic_wds: f64,
    pub holy_damage: f64,
    pub holy_wds: f64,
}

/// Contains the RPG-style attribute values that influence
/// derived stats, skill scaling, and other gameplay mechanics.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterSkillAttributes {
    pub vitality: f64,
    pub strength: f64,
    pub dexterity: f64,
    pub constitution: f64,
    pub intelligence: f64,
    pub faith: f64,
    pub luck: f64,
}

/// Contains various world-specific stats that are not directly related to gameplay.
/// Is needed for a handle on world effects and stats.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CharacterWorldStats {
    pub attack_range: f64,
}