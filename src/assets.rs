use std::borrow::Cow;

use anyhow::Result;
use gpui_kit::*;
use rust_embed::RustEmbed;

/// Embedded application assets (custom SVG icons) merged over the gpui-kit defaults.
#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
pub struct CawAssets;

impl AssetSource for CawAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if let Some(file) = Self::get(path) {
            return Ok(Some(file.data));
        }

        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut list: Vec<SharedString> = Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect();
        if let Ok(mut defaults) = gpui_kit::assets::Assets.list(path) {
            list.append(&mut defaults);
        }
        Ok(list)
    }
}
