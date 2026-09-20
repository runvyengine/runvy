//! Editor style and color definitions.
//!
//! This module centralizes all color constants and style configuration for the Runvy Editor.

#![allow(dead_code)]

use egui::{Color32, Style, TextStyle, Visuals};
use wgpu::Color;

/// Viewport background color (dark gray).
pub const VIEWPORT_BACKGROUND: Color32 = Color32::from_rgb(32, 32, 32);

/// Render target clear color (dark blue-gray).
/// Used for the 3D scene background.
pub const RENDER_CLEAR_COLOR: Color = Color {
    r: 0.00,
    g: 0.00,
    b: 0.00,
    a: 1.0,
};

/// Panel background color (hierarchy, inspector, content browser).
pub const PANEL_BACKGROUND: Color32 = Color32::from_rgb(34, 36, 42);

/// Component card background color (slightly lighter than panel background).
pub const COMPONENT_BACKGROUND: Color32 = Color32::from_rgb(44, 46, 53);

/// Error color for displaying errors and validation messages.
pub const ERROR_COLOR: Color32 = Color32::from_rgb(255, 80, 80);

/// Selection background color (blue highlight for selected items).
pub const SELECTION_BACKGROUND: Color32 = Color32::from_rgb(64, 87, 111);

/// Hover background color (subtle highlight for hovered items).
pub const HOVER_BACKGROUND: Color32 = Color32::from_rgb(44, 46, 53);

/// Border color for panels and separators.
pub const BORDER_COLOR: Color32 = Color32::from_rgb(14, 16, 22);

/// Text color for primary/normal text.
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);

/// Component title text color.
pub const COMPONENT_TITLE_COLOR: Color32 = Color32::from_rgb(220, 220, 220);

/// Accent color for buttons and interactive elements.
pub const ACCENT_COLOR: Color32 = Color32::from_rgb(70, 130, 220);

/// Panel dimensions.
pub mod panel_sizes {
    /// Initial viewport size (width, height).
    pub const INITIAL_VIEWPORT: (u32, u32) = (960, 540);

    /// Default bottom bar height.
    pub const BOTTOM_BAR_HEIGHT: f32 = 220.0;
}

/// UI spacing and layout constants.
pub mod spacing {
    /// Icon size in content browser (default).
    pub const CONTENT_ICON_SIZE: f32 = 48.0;

    /// Icon size for component icons in inspector.
    pub const COMPONENT_ICON_SIZE: f32 = 16.0;

    /// Corner radius for rounded rectangles (in u8 for egui).
    pub const CORNER_RADIUS: u8 = 4;
}

/// Font and text styling.
pub mod typography {
    use egui::{FontFamily, FontId};

    /// Default font size for body text.
    pub const BODY_FONT_SIZE: f32 = 14.0;

    /// Font size for headings.
    pub const HEADING_FONT_SIZE: f32 = 18.0;

    /// Font size for component titles.
    pub const COMPONENT_TITLE_FONT_SIZE: f32 = 14.0;

    /// Font size for small text (labels, captions).
    pub const SMALL_FONT_SIZE: f32 = 12.0;

    /// Font size for monospace text (console output, code).
    pub const MONOSPACE_FONT_SIZE: f32 = 13.0;

    /// Get the default font ID for body text.
    pub fn body_font() -> FontId {
        FontId::new(BODY_FONT_SIZE, FontFamily::Proportional)
    }

    /// Get the font ID for headings.
    pub fn heading_font() -> FontId {
        FontId::new(HEADING_FONT_SIZE, FontFamily::Proportional)
    }

    /// Get the font ID for component titles (bold style).
    pub fn component_title_font() -> FontId {
        FontId::new(
            COMPONENT_TITLE_FONT_SIZE,
            FontFamily::Name("Arial Bold".into()),
        )
    }

    /// Get the font ID for small text.
    pub fn small_font() -> FontId {
        FontId::new(SMALL_FONT_SIZE, FontFamily::Proportional)
    }

    /// Get the font ID for monospace text.
    pub fn monospace_font() -> FontId {
        FontId::new(MONOSPACE_FONT_SIZE, FontFamily::Monospace)
    }
}

/// Apply the editor's custom style to an egui context.
pub fn apply_editor_style(ctx: &egui::Context) {
    let mut style = Style {
        interaction: egui::style::Interaction {
            selectable_labels: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // Customize colors
    style.visuals = Visuals {
        dark_mode: true,
        override_text_color: Some(TEXT_PRIMARY),
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: PANEL_BACKGROUND,
                weak_bg_fill: PANEL_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, BORDER_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: HOVER_BACKGROUND,
                weak_bg_fill: HOVER_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, BORDER_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: HOVER_BACKGROUND,
                weak_bg_fill: HOVER_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, BORDER_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: SELECTION_BACKGROUND,
                weak_bg_fill: SELECTION_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, ACCENT_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: PANEL_BACKGROUND,
                weak_bg_fill: PANEL_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, ACCENT_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
        },
        selection: egui::style::Selection {
            bg_fill: SELECTION_BACKGROUND,
            stroke: egui::Stroke::new(1.0, ACCENT_COLOR),
        },
        window_fill: PANEL_BACKGROUND,
        panel_fill: PANEL_BACKGROUND,
        ..Default::default()
    };

    // Apply text styles
    style
        .text_styles
        .insert(TextStyle::Heading, typography::heading_font());
    style
        .text_styles
        .insert(TextStyle::Body, typography::body_font());
    style
        .text_styles
        .insert(TextStyle::Monospace, typography::monospace_font());
    style
        .text_styles
        .insert(TextStyle::Small, typography::small_font());
    style.text_styles.insert(
        TextStyle::Name("component_title".into()),
        typography::component_title_font(),
    );

    ctx.set_global_style(style);
}

/// Get the default egui visuals for dark mode with custom colors.
pub fn default_dark_visuals() -> Visuals {
    Visuals {
        dark_mode: true,
        override_text_color: Some(TEXT_PRIMARY),
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: PANEL_BACKGROUND,
                weak_bg_fill: PANEL_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, BORDER_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: HOVER_BACKGROUND,
                weak_bg_fill: HOVER_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, BORDER_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: HOVER_BACKGROUND,
                weak_bg_fill: HOVER_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, BORDER_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: SELECTION_BACKGROUND,
                weak_bg_fill: SELECTION_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, ACCENT_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: PANEL_BACKGROUND,
                weak_bg_fill: PANEL_BACKGROUND,
                bg_stroke: egui::Stroke::new(1.0, ACCENT_COLOR),
                fg_stroke: egui::Stroke::new(1.0, TEXT_PRIMARY),
                corner_radius: egui::CornerRadius::same(spacing::CORNER_RADIUS),
                expansion: 0.0,
            },
        },
        selection: egui::style::Selection {
            bg_fill: SELECTION_BACKGROUND,
            stroke: egui::Stroke::new(1.0, ACCENT_COLOR),
        },
        window_fill: PANEL_BACKGROUND,
        panel_fill: PANEL_BACKGROUND,
        ..Default::default()
    }
}
