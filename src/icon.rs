use std::{collections::HashMap, path::PathBuf, ffi::OsStr};

use walkdir::WalkDir;

use crate::search::xdg::xdg_directories;

/// Links an icon name to its path
#[derive(Debug, Default)]
pub struct IconCache(HashMap<String, Icon>);

#[derive(Debug)]
pub enum Icon {
    Svg(PathBuf),
    Other(PathBuf)
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
    pub fn new(icon_theme: &str) -> Self {
        let mut icon_dirs = xdg_directories("icons");
        for dir in &mut icon_dirs {
            dir.push('/');
            dir.push_str(icon_theme)
        }

        icon_dirs.push("/usr/share/pixmaps".to_owned());

        let mut cache = Self::default();

        for dir in icon_dirs {
            for file in WalkDir::new(&dir).into_iter().flatten() {
                if !file.metadata().unwrap().is_file() { continue }

                let Some(Some(name)) = file.path().file_stem().map(OsStr::to_str) else { continue }; // filter non utf-8 names
                if cache.0.contains_key(name) { continue } // filter already found icons

                cache.0.insert(name.to_owned(), file.into_path().into());
            }
        }

        cache
    }

    pub fn get(&self, icon: &str) -> Option<&Icon> {
        self.0.get(icon)
    }
}
