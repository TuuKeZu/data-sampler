use serde_derive::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Write};
use std::path::Path;
use toml;

#[derive(Deserialize, Serialize)]
pub struct Data {
    pub config: Config,
}

impl Data {
    pub fn as_toml(&self) -> String {
        toml::to_string(self).unwrap()
    }
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub pressure_field: String,
    pub displacement_field: String,
    pub min_max_field: String,
    pub pressure_threshold: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self { pressure_field: String::from("F_pri_pressure_bar"), displacement_field: String::from("Displacement_A_mm"), min_max_field: String::from("Displacement_A_mm"), pressure_threshold: 101. }
    }
}

pub fn get_config() -> Result<Config, std::io::Error> {
    let file_name = "Config.toml";

    file_exists(
        file_name,
        Data {
            config: Config::default(),
        }
        .as_toml(),
    )?;

    let contents = fs::read_to_string(file_name)?;
    let data: Data = toml::from_str(&contents)?;

    Ok(data.config)
}

pub fn dir_exists(path: &str) -> Result<(), std::io::Error> {
    if Path::is_dir(Path::new(path)) {
        return Ok(());
    }
    fs::create_dir(path)?;
    Ok(())
}

pub fn file_exists(path: &str, default_raw: String) -> Result<(), std::io::Error> {
    if Path::is_file(Path::new(path)) {
        return Ok(());
    }

    let mut file = File::create(path)?;
    file.write_all(default_raw.as_bytes())?;
    Ok(())
}