//! Categories: a folder, file-extension rules, a speed limit and a cap on
//! running jobs, shared by every job in them. See docs/decisions/0011.

use std::fmt;
use std::path::PathBuf;

use reqwest::Url;

use crate::error::Error;
use crate::filename;

/// Identifies a category. Categories are few and the user names them, so this
/// is a plain number rather than a time-based ID like [`crate::JobId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CategoryId(pub u32);

impl CategoryId {
    /// The built-in category: it can't be removed, jobs that match no other
    /// category go there, and jobs of a removed category move there.
    pub const GENERAL: CategoryId = CategoryId(1);
}

impl fmt::Display for CategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// What the user can set on a category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorySettings {
    /// Unique, ignoring case.
    pub name: String,
    /// Where its jobs are saved unless the job names a folder. `None`: jobs
    /// must name one.
    pub folder: Option<PathBuf>,
    /// File extensions that put a new download in this category, lowercase
    /// and without the dot. Each extension belongs to one category at most.
    pub extensions: Vec<String>,
    /// Bytes per second for all its jobs together; 0 = none.
    pub speed_limit: u64,
    /// How many of its jobs may download at once; 0 = no cap of its own
    /// (the global one still applies).
    pub max_concurrent: usize,
}

impl CategorySettings {
    /// A category with no folder, no rules and no limits.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            folder: None,
            extensions: Vec::new(),
            speed_limit: 0,
            max_concurrent: 0,
        }
    }

    /// Trims the name, and lowercases, de-dots, sorts and de-duplicates the
    /// extensions, so `".MKV"` and `"mkv"` are the same rule.
    pub(crate) fn normalized(mut self) -> Result<Self, Error> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() {
            return Err(Error::InvalidCategoryName);
        }
        if let Some(folder) = &self.folder
            && folder.to_str().is_none()
        {
            return Err(Error::NonUtf8Path(folder.clone()));
        }
        for ext in &mut self.extensions {
            let clean = ext.trim().trim_start_matches('.').to_lowercase();
            if clean.is_empty() || clean.contains(['.', '/', '\\']) {
                return Err(Error::InvalidExtension(ext.clone()));
            }
            *ext = clean;
        }
        self.extensions.sort();
        self.extensions.dedup();
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: CategoryId,
    pub settings: CategorySettings,
}

/// Where a link would go if added without a category, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matched {
    pub category: CategoryId,
    /// The extension the URL suggests, if any. `None`, or one no category
    /// lists, means General.
    pub extension: Option<String>,
}

/// The extension of the file name `url` suggests, lowercase, if it has one.
pub(crate) fn url_extension(url: &Url) -> Option<String> {
    filename::extension(&filename::from_url(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_normalized() {
        let settings = CategorySettings {
            extensions: vec![".MKV".into(), "mp4".into(), " mkv ".into()],
            ..CategorySettings::named("  Videos ")
        };
        let settings = settings.normalized().unwrap();
        assert_eq!(settings.name, "Videos");
        assert_eq!(settings.extensions, ["mkv", "mp4"]);
    }

    #[test]
    fn bad_names_and_extensions_are_refused() {
        assert!(CategorySettings::named(" ").normalized().is_err());
        for bad in ["", ".", "tar.gz", "a/b"] {
            let settings = CategorySettings {
                extensions: vec![bad.into()],
                ..CategorySettings::named("x")
            };
            assert!(settings.normalized().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn extension_comes_from_the_url_path() {
        let ext = |url: &str| url_extension(&Url::parse(url).unwrap());
        assert_eq!(
            ext("https://a.com/films/Movie.MKV?token=x"),
            Some("mkv".into())
        );
        assert_eq!(ext("https://a.com/download.php?id=3"), Some("php".into()));
        assert_eq!(ext("https://a.com/files/README"), None);
        assert_eq!(ext("https://a.com/.hidden"), None);
        assert_eq!(ext("https://a.com/"), None);
    }
}
