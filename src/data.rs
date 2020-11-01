// The structure of data saved in between app launches

use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fs::File;
use std::path::{PathBuf};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Data {
    pub discord_token: Option<String>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            discord_token: None,
        }
    }
}

impl Data {
    pub fn load() -> Result<Self, String> {
        // check if file exists
        let filepath = get_path().join("data");
        let file = if filepath.exists() {
            // Loads the data from the system
            match File::open(filepath) {
                Ok(f) => f,
                Err(e) => return Err(format!("{}", e)),
            }
        } else {
            // Just return the default data
            return Ok(Self::default());
        };

        match bincode::deserialize_from(file) {
            Ok(d) => Ok(d),
            Err(e) => Err(format!("{}", e)),
        }
    }
    pub fn save(&self) -> Result<(), String> {
        // make sure the directory exists
        std::fs::create_dir_all(get_path()).unwrap();

        // Saves the data to the system
        let file = match File::create(get_path().join("data")) {
            Ok(f) => f,
            Err(e) => return Err(format!("{}", e)),
        };

        match bincode::serialize_into(file, self) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

fn get_path() -> PathBuf {
    directories_next::ProjectDirs::from("", "", "Oxycord")
        .unwrap()
        .data_dir()
        .to_path_buf()
}
