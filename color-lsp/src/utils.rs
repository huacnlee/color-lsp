use tower_lsp::lsp_types;

/// Convert lsp_types::Color to markdown to list other color formats (HSLA, HEX, RGBA)
/// e.g.
///
/// Colorspace Formats:
///
/// - #EECC00
/// - #EECC00FF
/// - hsla(51.4, 100%, 46.7%, 100%)
/// - hsla(0.143, 1., 0.467, 1.)
/// - rgba(238, 204, 0, 100%)
/// - rgba(0.933, 0.8, 0., 1.)
pub(crate) fn color_summary(color: lsp_types::Color) -> String {
    let r = (color.red * 255.0).round() as u8;
    let g = (color.green * 255.0).round() as u8;
    let b = (color.blue * 255.0).round() as u8;
    let a = (color.alpha * 255.0).round() as u8;

    let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
    let hex_alpha = format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        r,
        g,
        b,
        (color.alpha * 255.0).round() as u8
    );
    let hsla = rgba_to_hsla(r, g, b, a);
    let hsla_percent = format!(
        "hsla({}, {}%, {}%, {}%)",
        format_trimmed(hsla.0 * 360., 1, false),
        format_trimmed(hsla.1 * 100., 1, true),
        format_trimmed(hsla.2 * 100., 1, true),
        format_trimmed(hsla.3 * 100., 1, true),
    );
    let hsla_float = format!(
        "hsla({}, {}, {}, {})",
        format_trimmed(hsla.0, 3, false),
        format_trimmed(hsla.1, 3, false),
        format_trimmed(hsla.2, 3, false),
        format_trimmed(hsla.3, 3, false)
    );

    let rgba = format!("rgba({}, {}, {}, {}%)", r, g, b, a / 255 * 100);
    let rgba_float = format!(
        "rgba({}, {}, {}, {})",
        format_trimmed(color.red, 3, false),
        format_trimmed(color.green, 3, false),
        format_trimmed(color.blue, 3, false),
        format_trimmed(color.alpha, 3, false)
    );

    // let color_img = format!(
    //     "![Color](https://singlecolorimage.com/get/{}/128x32)\n",
    //     &hex[1..]
    // );

    let color_link = format!("\n[Color Picker](https://colorpicker.dev/{})", &hex);

    vec![
        "Colorspace Formats:\n".to_string(),
        format!("- {}", hex),
        format!("- {}", hex_alpha),
        format!("- {}", hsla_percent),
        format!("- {}", hsla_float),
        format!("- {}", rgba),
        format!("- {}", rgba_float),
        color_link,
    ]
    .join("\n")
}

pub(crate) fn format_trimmed(x: f32, precision: usize, trim_end_dot: bool) -> String {
    let mut s = format!("{:.1$}", x, precision)
        .trim_end_matches('0')
        .to_string();

    if trim_end_dot {
        s = s.trim_end_matches(".").to_string();
    }

    s
}

pub(crate) fn rgba_to_hsla(r: u8, g: u8, b: u8, a: u8) -> (f32, f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let a = a as f32 / 255.0;

    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    let l = (max + min) / 2.0;

    let s = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * l - 1.0).abs())
    };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    (h / 360., s, l, a)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use tower_lsp::lsp_types;

    #[test]
    fn test_color_summary() {
        let color = lsp_types::Color {
            red: 0.933,
            green: 0.8,
            blue: 0.0,
            alpha: 1.0,
        };

        let summary = super::color_summary(color);
        assert_eq!(
            summary,
            indoc! {r#"
                Colorspace Formats:

                - #EECC00
                - #EECC00FF
                - hsla(51.4, 100%, 46.7%, 100%)
                - hsla(0.143, 1., 0.467, 1.)
                - rgba(238, 204, 0, 100%)
                - rgba(0.933, 0.8, 0., 1.)

                [Color Picker](https://colorpicker.dev/#EECC00)"#}
        );
    }

    #[test]
    fn test_rgba_to_hsla() {
        let (h, s, l, a) = super::rgba_to_hsla(238, 204, 0, 255);
        assert!((h - 0.143).abs() < 0.001);
        assert!((s - 1.0).abs() < 0.001);
        assert!((l - 0.467).abs() < 0.001);
        assert!((a - 1.0).abs() < 0.001);
    }
}
