use gpui_kit::component::{IconName, IconNamed};
use gpui_kit::*;

/// Application icons: a mix of gpui-kit bundled Lucide icons and Caw own SVGs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CawIcon {
    /// App logo drawn in the UI (titlebar, sidebar): a 16-24px crop of `caw.svg`.
    App,
    Music,
    Disc,
    ListMusic,
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Shuffle,
    Repeat,
    Repeat1,
    Volume2,
    VolumeX,
    Locate,
    Folder,
    Settings,
    Plus,
    Trash,
    Refresh,
    ArrowLeft,
    Close,
    User,
}

impl IconNamed for CawIcon {
    fn path(self) -> SharedString {
        match self {
            Self::App => "icons/caw-glyph.svg".into(),
            Self::Music => "icons/music.svg".into(),
            Self::Disc => "icons/disc-3.svg".into(),
            Self::ListMusic => "icons/list-music.svg".into(),
            Self::Play => IconName::Play.path(),
            Self::Pause => IconName::Pause.path(),
            Self::SkipBack => "icons/skip-back.svg".into(),
            Self::SkipForward => "icons/skip-forward.svg".into(),
            Self::Shuffle => "icons/shuffle.svg".into(),
            Self::Repeat => "icons/repeat.svg".into(),
            Self::Repeat1 => "icons/repeat-1.svg".into(),
            Self::Volume2 => "icons/volume-2.svg".into(),
            Self::VolumeX => "icons/volume-x.svg".into(),
            Self::Locate => "icons/locate.svg".into(),
            Self::Folder => IconName::Folder.path(),
            Self::Settings => IconName::Settings.path(),
            Self::Plus => IconName::Plus.path(),
            Self::Trash => IconName::Delete.path(),
            Self::Refresh => IconName::RefreshCw.path(),
            Self::ArrowLeft => IconName::ArrowLeft.path(),
            Self::Close => IconName::Close.path(),
            Self::User => IconName::User.path(),
        }
    }
}
