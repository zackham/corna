use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_position")]
    pub position: Position,

    #[serde(default = "default_margins")]
    pub margins: Margins,

    #[serde(default = "default_size")]
    pub collapsed_size: Size,

    #[serde(default = "default_expanded_size")]
    pub expanded_size: Size,

    #[serde(default = "default_theme")]
    pub theme: Theme,

    #[serde(default = "default_fps_cap")]
    pub fps_cap: u32,

    #[serde(default)]
    pub animations_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub anchor: Anchor,
    pub exclusive_zone: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margins {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub background: String,
    pub foreground: String,
    pub accent: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            position: default_position(),
            margins: default_margins(),
            collapsed_size: default_size(),
            expanded_size: default_expanded_size(),
            theme: default_theme(),
            fps_cap: default_fps_cap(),
            animations_enabled: true,
        }
    }
}

fn default_position() -> Position {
    Position {
        anchor: Anchor::TopRight,
        exclusive_zone: 0,
    }
}

fn default_margins() -> Margins {
    Margins {
        top: 8,
        right: 8,
        bottom: 8,
        left: 8,
    }
}

fn default_size() -> Size {
    Size {
        width: 150,
        height: 60,  // Larger for cleaner digit rendering
    }
}

fn default_expanded_size() -> Size {
    Size {
        width: 300,
        height: 120,
    }
}

fn default_theme() -> Theme {
    Theme {
        background: "#1a1a1a".to_string(),
        foreground: "#ffffff".to_string(),
        accent: "#4a9eff".to_string(),
    }
}

fn default_fps_cap() -> u32 {
    60
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        let config_path = config_dir.join("corna").join("config.toml");

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        let config_dir = config_dir.join("corna");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;

        Ok(())
    }
}