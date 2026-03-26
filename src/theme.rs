use gpui::Rgba;

const fn hex(hex: u32) -> Rgba {
    let [_, r, g, b] = hex.to_be_bytes();
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub const BG_BASE: Rgba = hex(0x1e1e2e);
pub const BG_SURFACE: Rgba = hex(0x181825);
pub const BG_OVERLAY: Rgba = hex(0x313244);
pub const TEXT_PRIMARY: Rgba = hex(0xcdd6f4);
pub const TEXT_MUTED: Rgba = hex(0x6c7086);
pub const ACCENT: Rgba = hex(0x89b4fa);

// Tool call colors
pub const BG_TOOL: Rgba = hex(0x1a1a2e);
pub const BORDER_TOOL: Rgba = hex(0x45475a);
pub const TEXT_TOOL_NAME: Rgba = hex(0xf9e2af);
pub const TEXT_TOOL_INPUT: Rgba = hex(0xa6adc8);
pub const BG_TOOL_OUTPUT: Rgba = hex(0x11111b);
pub const TEXT_ERROR: Rgba = hex(0xf38ba8);
