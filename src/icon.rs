use std::{collections::HashMap, path::{PathBuf, Path}};

use walkdir::WalkDir;

use crate::xdg_utils::xdg_directories;

/// Distinguishes between a direct path to an icon, and an icon identifier that needs to be searched in IconCache.
#[derive(Debug, Clone)]
pub enum IconPath {
    Name(String),
    Path(Icon)
}

/// Links an icon name to its path
#[derive(Debug, Default)]
pub struct IconCache(HashMap<String, Icon>);

#[derive(Debug, Clone)]
pub enum Icon {
    Svg(PathBuf),
    Other(PathBuf)
}

impl IconPath {
    pub fn new(value: String, cwd: Option<&Path>) -> Self {
        let process_cwd = std::env::current_dir().ok();
        let cwd = cwd.or(process_cwd.as_deref());

        if Path::new(&value).is_absolute() {
            IconPath::Path(PathBuf::from(value).into())
        } else if Path::new(&value).starts_with("./") && cwd.is_some() {
            IconPath::Path(cwd.unwrap().join(value).into())
        } else {
            IconPath::Name(value)
        }
    }
}

impl From<PathBuf> for Icon {
    fn from(value: PathBuf) -> Self {
        if value.extension().map_or(false, |ext| ext == "svg") {
            Self::Svg(value)
        } else {
            Self::Other(value)
        }
    }
}

impl IconCache {
    pub fn new(icon_themes: &[String]) -> Self {
        let icon_dirs = xdg_directories("icons");
        // for every xdg directory, add icon theme, by order of preference
        let mut icon_dirs: Vec<_> = icon_themes.iter()
            .flat_map(|theme| icon_dirs.iter().map(move |dir| dir.join(theme)))
            .collect();

        icon_dirs.push("/usr/share/pixmaps".into());

        let mut cache = Self::default();

        for dir in icon_dirs {
            for file in WalkDir::new(&dir).follow_links(true).into_iter().flatten() {
                if !file.metadata().unwrap().is_file() { continue }

                let Some(Some(name)) = file.path().file_stem().map(|x| x.to_str()) else { continue }; // filter non utf-8 names
                if cache.0.contains_key(name) { continue } // filter already found icons

                cache.0.insert(name.to_owned(), file.into_path().into());
            }
        }

        cache
    }

    pub fn get<'a>(&'a self, icon: &'a IconPath) -> Option<&'a Icon> {
        match icon {
            IconPath::Name(icon) => self.0.get(icon),
            IconPath::Path(icon) => Some(icon)
        }
    }
}
