//! Auto-scrolling single-line text for labels wider than their box.
//!
//! Short text renders as a static ellipsized line; long text scrolls at a
//! constant speed with a short hold at each end. The row keeps a fixed height,
//! so a changing track title never shifts the layout around it.
//!
//! Motion goes through gpui AnimationExt, which means the platform reduced
//! motion setting turns the scroll into a static first frame automatically.

use std::time::Duration;

use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

/// Fraction of each loop spent holding still at the two ends.
const HOLD_FRACTION: f32 = 0.15;
/// Gap between the two copies of the text while scrolling.
const GAP: Pixels = px(48.);
/// Scroll speed in pixels per second (matches the old Vue marquee).
const SPEED: f32 = 30.0;

/// A fixed-height, single-line text label that scrolls when it overflows.
#[derive(IntoElement)]
pub struct MarqueeText {
    text: SharedString,
    width: Pixels,
    height: Pixels,
    font_size: Pixels,
    font_weight: FontWeight,
    color: Option<Hsla>,
}

impl MarqueeText {
    /// Create a marquee whose visible box is width pixels wide.
    pub fn new(text: impl Into<SharedString>, width: Pixels) -> Self {
        Self {
            text: text.into(),
            width,
            height: px(20.),
            font_size: px(14.),
            font_weight: FontWeight::default(),
            color: None,
        }
    }

    /// Visible row height; keep it fixed so the surrounding layout is stable.
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    pub fn font_size(mut self, size: Pixels) -> Self {
        self.font_size = size;
        self
    }

    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self
    }

    pub fn text_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for MarqueeText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let family = theme.font_family.clone();
        let color = self.color.unwrap_or(theme.foreground);

        // Single line only: a title containing newlines would panic the shaper.
        let text: SharedString = self.text.replace(['\n', '\r'], " ").into();

        let mut font = gpui_kit::font(family.clone());
        font.weight = self.font_weight;
        let run = TextRun {
            len: text.len(),
            font,
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let text_width = window
            .text_system()
            .shape_line(text.clone(), self.font_size, &[run], None)
            .width();

        let container = h_flex()
            .w(self.width)
            .h(self.height)
            .flex_shrink_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_size(self.font_size)
            .font_family(family)
            .font_weight(self.font_weight)
            .text_color(color);

        // Short label or reduced-motion: static single line, ellipsized if our
        // measurement was slightly optimistic relative to the painted layout.
        if cx.reduce_motion() || text_width + px(2.) <= self.width {
            return container
                .child(div().w_full().min_w(px(0.)).truncate().child(text))
                .into_any_element();
        }

        let distance = text_width + GAP;
        let travel_seconds = distance.as_f32() / SPEED;
        let period =
            Duration::from_secs_f32((travel_seconds / (1.0 - 2.0 * HOLD_FRACTION)).max(1.0));
        let id = ElementId::Name(
            format!(
                "marquee-{}-{}-{}",
                self.width.as_f32(),
                text_width.as_f32(),
                text
            )
            .into(),
        );
        let offset_text = text.clone();

        let track = h_flex()
            .relative()
            .flex_shrink_0()
            .gap_0()
            .child(div().flex_shrink_0().child(text))
            .child(div().flex_shrink_0().w(GAP))
            .child(div().flex_shrink_0().child(offset_text))
            .with_animation(
                id,
                Animation::new(period)
                    .repeat()
                    .with_max_fps(30.0)
                    .with_easing(marquee_easing),
                move |this, progress| this.left(px(-distance.as_f32() * progress)),
            );

        container.child(track).into_any_element()
    }
}

/// Hold at both ends, move at constant speed in between.
fn marquee_easing(progress: f32) -> f32 {
    if progress <= HOLD_FRACTION {
        0.0
    } else if progress >= 1.0 - HOLD_FRACTION {
        1.0
    } else {
        (progress - HOLD_FRACTION) / (1.0 - 2.0 * HOLD_FRACTION)
    }
}
