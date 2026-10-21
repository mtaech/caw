//! System font enumeration for the settings font picker.
//!
//! The picker lists every family the text system knows about (fontconfig on Linux,
//! the platform font database elsewhere), so there is no hard-coded font list.

use gpui_kit::component::searchable_list::SearchableListItem;
use gpui_kit::*;

/// One selectable system font family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontItem {
    family: SharedString,
}

impl FontItem {
    pub fn new(family: impl Into<SharedString>) -> Self {
        Self {
            family: family.into(),
        }
    }

    pub fn family(&self) -> &SharedString {
        &self.family
    }
}

impl SearchableListItem for FontItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.family.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.family
    }
}

/// Every font family installed on the system, sorted case-insensitively.
///
/// `current` is kept in the list even if the text system does not enumerate it
/// (for example a family configured on another machine), so the picker can always
/// show the committed value.
pub fn system_font_items(cx: &App, current: &str) -> Vec<FontItem> {
    let mut families: Vec<SharedString> = cx
        .text_system()
        .all_font_names()
        .into_iter()
        .map(SharedString::from)
        .collect();

    families.sort_by_cached_key(|family| family.to_lowercase());
    families.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    if !current.is_empty()
        && !families
            .iter()
            .any(|family| family.as_ref().eq_ignore_ascii_case(current))
    {
        families.insert(0, SharedString::from(current));
    }

    families.into_iter().map(FontItem::new).collect()
}
