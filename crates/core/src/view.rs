//! Saved views: a name plus the filters and the order of the downloads list,
//! kept by the daemon so every device shows the same ones. The engine only
//! keeps them; front ends do the filtering. See docs/decisions/0026.

use std::fmt;

use crate::category::CategoryId;
use crate::error::Error;
use crate::job::JobState;

/// Identifies a saved view. Like [`CategoryId`], a plain number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewId(pub u32);

impl fmt::Display for ViewId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// How recently a download was added. "Today" starts at midnight where the
/// list is shown, so the front end decides what it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddedWithin {
    #[default]
    Any,
    Today,
    /// The past 7 days.
    Week,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// By ID, which is by creation time.
    #[default]
    Added,
    Name,
    Size,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

/// The states a view can pick, in the order lists of them are kept.
/// `Connecting` isn't one: it counts as `Downloading` (it only lasts until
/// the server answers).
const STATES: [JobState; 8] = [
    JobState::Downloading,
    JobState::Retrying,
    JobState::Queued,
    JobState::Scheduled,
    JobState::Paused,
    JobState::Failed,
    JobState::Cancelled,
    JobState::Completed,
];

/// What the user can set on a view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSettings {
    /// Unique, ignoring case.
    pub name: String,
    /// Downloads in one of these states; `None`: any state. An empty list
    /// shows nothing.
    pub states: Option<Vec<JobState>>,
    /// Downloads in one of these categories; `None`: any category.
    pub categories: Option<Vec<CategoryId>>,
    /// Part of the file name, in any case; empty: any name.
    pub name_contains: String,
    pub added: AddedWithin,
    pub sort: SortKey,
    pub direction: SortDirection,
    /// Grouped by status, each group sorted.
    pub grouped: bool,
}

impl ViewSettings {
    /// A view of every download, oldest first, grouped by status (the
    /// downloads list's defaults).
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            states: None,
            categories: None,
            name_contains: String::new(),
            added: AddedWithin::Any,
            sort: SortKey::Added,
            direction: SortDirection::Ascending,
            grouped: true,
        }
    }

    /// Trims the name and the text, counts `Connecting` as `Downloading`, and
    /// puts the lists in a fixed order without repeats, so the same choice
    /// is always saved the same way.
    pub(crate) fn normalized(mut self) -> Result<Self, Error> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() {
            return Err(Error::InvalidViewName);
        }
        self.name_contains = self.name_contains.trim().to_owned();
        if let Some(states) = &mut self.states {
            for state in states.iter_mut() {
                if *state == JobState::Connecting {
                    *state = JobState::Downloading;
                }
            }
            *states = STATES
                .into_iter()
                .filter(|state| states.contains(state))
                .collect();
        }
        if let Some(categories) = &mut self.categories {
            categories.sort();
            categories.dedup();
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub id: ViewId,
    pub settings: ViewSettings,
}

impl AddedWithin {
    /// As stored in the database.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AddedWithin::Any => "any",
            AddedWithin::Today => "today",
            AddedWithin::Week => "week",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        [Self::Any, Self::Today, Self::Week]
            .into_iter()
            .find(|added| added.as_str() == text)
    }
}

impl SortKey {
    /// As stored in the database.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SortKey::Added => "added",
            SortKey::Name => "name",
            SortKey::Size => "size",
            SortKey::Progress => "progress",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        [Self::Added, Self::Name, Self::Size, Self::Progress]
            .into_iter()
            .find(|key| key.as_str() == text)
    }
}

impl SortDirection {
    /// As stored in the database.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SortDirection::Ascending => "asc",
            SortDirection::Descending => "desc",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        [Self::Ascending, Self::Descending]
            .into_iter()
            .find(|direction| direction.as_str() == text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_are_normalized() {
        let settings = ViewSettings {
            states: Some(vec![
                JobState::Failed,
                JobState::Connecting,
                JobState::Retrying,
                JobState::Downloading,
            ]),
            categories: Some(vec![CategoryId(4), CategoryId(1), CategoryId(4)]),
            name_contains: " iso ".into(),
            ..ViewSettings::named("  ISOs ")
        };
        let settings = settings.normalized().unwrap();
        assert_eq!(settings.name, "ISOs");
        assert_eq!(settings.name_contains, "iso");
        assert_eq!(
            settings.states,
            Some(vec![
                JobState::Downloading,
                JobState::Retrying,
                JobState::Failed
            ])
        );
        assert_eq!(
            settings.categories,
            Some(vec![CategoryId(1), CategoryId(4)])
        );
    }

    #[test]
    fn an_empty_list_stays_empty_and_a_blank_name_is_refused() {
        let settings = ViewSettings {
            states: Some(Vec::new()),
            ..ViewSettings::named("Nothing")
        };
        assert_eq!(settings.normalized().unwrap().states, Some(Vec::new()));
        assert!(ViewSettings::named(" ").normalized().is_err());
    }

    #[test]
    fn names_round_trip() {
        for added in [AddedWithin::Any, AddedWithin::Today, AddedWithin::Week] {
            assert_eq!(AddedWithin::parse(added.as_str()), Some(added));
        }
        for key in [
            SortKey::Added,
            SortKey::Name,
            SortKey::Size,
            SortKey::Progress,
        ] {
            assert_eq!(SortKey::parse(key.as_str()), Some(key));
        }
        for direction in [SortDirection::Ascending, SortDirection::Descending] {
            assert_eq!(SortDirection::parse(direction.as_str()), Some(direction));
        }
        assert_eq!(SortKey::parse("colour"), None);
    }
}
