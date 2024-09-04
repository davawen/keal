use std::{collections::HashMap, sync::OnceLock};

use indexmap::IndexMap;

use crate::{xdg_utils::config_dir, ini_parser::Ini};

// WARN: When adding fields to the config, remember to set them in `add_from_string`!

// This should probably use serde, but the `serde_ini` crate seems suboptimal for sections, and the current custom parser works well enough
#[derive(Debug)]
pub struct Config {
    pub font: String,
    pub font_size: f32,
    pub icon_theme: Vec<String>,
    pub usage_frequency: bool,
    pub terminal_path: String,
    pub placeholder_text: String,
    pub default_plugins: Vec<String>,
    pub plugin_overrides: HashMap<String, Override>,
    pub plugin_configs: HashMap<String, IndexMap<String, String>>
}

#[derive(Default, Debug)]
pub struct Override {
    pub prefix: Option<String>,
    pub icon: Option<String>,
    pub comment: Option<String>
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: String::new(),
            font_size: 0.0,
            icon_theme: vec![],
            terminal_path: String::new(),
            placeholder_text: String::new(),
            usage_frequency: false,
            default_plugins: Vec::new(),
            plugin_overrides: Default::default(),
            plugin_configs: Default::default()
        }
    }
}

pub trait FrontendConfig {
    /// The sections in the INI file used by this config
    fn sections(&self) -> &'static [&'static str];

    /// Use a field from the INI file
    fn add_field(&mut self, field: (String, String));
}

static CONFIG: OnceLock<Config> = OnceLock::new();
pub fn config() -> &'static Config {
    CONFIG.get().expect("config should have been initialized in main")
}

// Since the name of the field in the ini is the same as in the `Config` struct, we can match it directly.
// This is what `stringify!($name)` is doing.
// The type checker can work backwards from `$config.$name = v` to find what type is to be parsed, and what implementation of `MyFromStr` should be called.
// Pretty cool!
#[macro_export]
macro_rules! parse_fields {
    ($config:expr, $field:expr, ($($name:ident),+)) => {
        match $field.0.as_str() {
            $(
                stringify!($name) => match $field.1.my_parse() {
                    Ok(v) => $config.$name = v,
                    Err(e) => eprintln!("error with field `{}`: {}: `{}`", stringify!($name), e, $field.1)
                }
            ),+
            _ => ()
        }
    };
}

impl Config {
    pub fn init<T: FrontendConfig>(frontend: &mut T) -> &'static Self {
        CONFIG.get_or_init(|| Self::load(frontend))
    }

    /// Loads the default included configuration (in public/default-config.ini)
    fn default_config<T: FrontendConfig>(frontend: &mut T) -> Self {
        // SAFETY: the default config needs to have every field filled in
        let mut config = Config::default();
        config.add_from_string(frontend, include_str!("../../public/default-config.ini").to_owned());
        config
    }

    fn add_from_string<T: FrontendConfig>(&mut self, frontend: &mut T, content: String) {
        let mut file = Ini::from_string(content, &['#', ';']);

        for field in file.section("keal").into_iter().flat_map(|s| s.iter()) {
            parse_fields!(self, field, (
                font, font_size, icon_theme, usage_frequency, terminal_path, placeholder_text, default_plugins
            ));
        }

        for &section in frontend.sections() {
            for field in file.remove_section(section).into_iter().flat_map(|s| s.into_iter()) {
                frontend.add_field(field);
            }
        }

        for (name, section) in file.into_sections() {
            let Some((name, kind)) = name.rsplit_once('.') else { continue };

            match kind {
                "plugin" => {
                    let mut over = Override::default();
                    for field in section.iter() {
                        parse_fields!(over, field, (
                            prefix, icon, comment
                        ))
                    }
                    self.plugin_overrides.insert(name.to_owned(), over);
                }
                "config" => {
                    self.plugin_configs.insert(name.to_owned(), section.into_map());
                }
                _ => eprintln!("unknown plugin configuration kind: `{name}.{kind}`")
            }
        }
    }

    fn load<T: FrontendConfig>(frontend: &mut T) -> Self {
        let mut config = Config::default_config(frontend);

        let Ok(mut config_path) = config_dir() else { return config };
        config_path.push("config.ini");

        let Ok(content) = std::fs::read_to_string(config_path) else { return config };

        config.add_from_string(frontend, content);
        config
    }
}

trait MyFromStr<T> {
    fn my_parse(&self) -> Result<T, &str>;
}

impl<T> MyFromStr<Vec<T>> for str where str: MyFromStr<T> {
    fn my_parse(&self) -> Result<Vec<T>, &str> {
        self.split(',').map(|x| x.my_parse()).collect::<Result<_, _>>()
    }
}

impl<T> MyFromStr<Option<T>> for str where str: MyFromStr<T> {
    fn my_parse(&self) -> Result<Option<T>, &str> {
        Ok(Some(self.my_parse()?))
    }
}

impl MyFromStr<bool> for str {
    fn my_parse(&self) -> Result<bool, &'static str> {
        match self {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err("invalid boolean")
        }
    }
}

impl MyFromStr<String> for str {
    fn my_parse(&self) -> Result<String, &'static str> {
        Ok(self.to_owned())
    }
}

impl MyFromStr<f32> for str {
    fn my_parse(&self) -> Result<f32, &'static str> {
        self.parse().map_err(|_| "couldn't parse number")
    }
}
