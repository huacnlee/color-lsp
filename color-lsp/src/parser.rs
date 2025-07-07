use csscolorparser::{Color, ParseColorError};
use tower_lsp::lsp_types;

#[derive(Debug, Clone)]
pub struct ColorNode {
    pub color: Color,
    pub matched: String,
    /// Line, Column (1-based) of the node in the text.
    pub loc: (usize, usize),
}

impl Eq for ColorNode {}
impl PartialEq for ColorNode {
    fn eq(&self, other: &Self) -> bool {
        self.matched == other.matched
            && self.loc == other.loc
            && self.color.to_css_hex() == other.color.to_css_hex()
    }
}

impl ColorNode {
    fn new(matched: &str, color: Color, line: usize, col: usize) -> Self {
        Self {
            matched: matched.to_string(),
            loc: (line, col),
            color,
        }
    }

    #[allow(unused)]
    fn must_parse(matched: &str, line: usize, col: usize) -> Self {
        let color = try_parse_color(matched).expect("The `matched` should be a valid CSS color");
        Self::new(matched, color, line, col)
    }

    pub(crate) fn lsp_color(&self) -> lsp_types::Color {
        lsp_types::Color {
            red: self.color.r,
            green: self.color.g,
            blue: self.color.b,
            alpha: self.color.a,
        }
    }
}

fn try_parse_color(s: &str) -> Result<Color, ParseColorError> {
    if let Ok(color) = try_parse_gpui_color(s) {
        return Ok(color);
    }

    csscolorparser::parse(s)
}

/// Try to parse gpui color that values are 0..1
fn try_parse_gpui_color(s: &str) -> Result<Color, ParseColorError> {
    let s = s.trim();

    /// Parse and ensure all value in 0..1
    fn parse_f8(s: &str) -> Option<f32> {
        s.parse()
            .ok()
            .and_then(|v| (0.0..=1.0).contains(&v).then_some(v))
    }

    if let (Some(idx), Some(s)) = (s.find('('), s.strip_suffix(')')) {
        let fname = &s[..idx].trim_end();
        let mut params = s[idx + 1..]
            .split(',')
            .flat_map(str::split_ascii_whitespace);

        let (Some(val0), Some(val1), Some(val2)) = (params.next(), params.next(), params.next())
        else {
            return Err(ParseColorError::InvalidFunction);
        };

        let alpha = if let Some(a) = params.next() {
            if let Some(v) = parse_f8(a) {
                v.clamp(0.0, 1.0)
            } else {
                return Err(ParseColorError::InvalidFunction);
            }
        } else {
            1.0
        };

        if params.next().is_some() {
            return Err(ParseColorError::InvalidFunction);
        }

        if fname.eq_ignore_ascii_case("rgb") || fname.eq_ignore_ascii_case("rgba") {
            if let (Some(v0), Some(v1), Some(v2)) = (parse_f8(val0), parse_f8(val1), parse_f8(val2))
            {
                return Ok(Color::new(v0, v1, v2, alpha));
            } else {
                return Err(ParseColorError::InvalidFunction);
            }
        } else if fname.eq_ignore_ascii_case("hsl") || fname.eq_ignore_ascii_case("hsla") {
            if let (Some(v0), Some(v1), Some(v2)) = (parse_f8(val0), parse_f8(val1), parse_f8(val2))
            {
                return Ok(Color::from_hsla(v0 * 360.0, v1, v2, alpha));
            } else {
                return Err(ParseColorError::InvalidFunction);
            }
        }
    }

    Err(ParseColorError::InvalidUnknown)
}

fn is_hex_char(c: &char) -> bool {
    matches!(c, '#' | 'a'..='f' | 'A'..='F' | '0'..='9')
}

pub(super) fn parse(text: &str) -> Vec<ColorNode> {
    let mut nodes = Vec::new();

    for (ix, line_text) in text.lines().enumerate() {
        let line_len = line_text.len();
        let mut offset = 0;
        let mut token = String::new();
        while offset < line_text.len() {
            let c = line_text.chars().nth(offset).unwrap_or(' ');
            match c {
                '#' => {
                    token.clear();

                    // Find the hex color code
                    let hex = line_text[offset..]
                        .chars()
                        .take_while(is_hex_char)
                        .take(9)
                        .collect::<String>();
                    if let Some(node) = match_color(&hex, ix, offset) {
                        nodes.push(node);
                        offset += hex.len();
                        continue;
                    }
                }
                'a'..='z' | 'A'..='Z' | '(' => {
                    token.push(c);

                    match token.as_ref() {
                        // Ref https://github.com/mazznoer/csscolorparser-rs
                        "hsl(" | "hsla(" | "rgb(" | "rgba(" | "hwb(" | "hwba(" | "oklab("
                        | "oklch(" | "lab(" | "lch(" | "hsv(" => {
                            // Find until the closing parenthesis
                            let end = line_text[offset..]
                                .chars()
                                .position(|c| c == ')')
                                .unwrap_or(0);
                            let token_offset = offset.saturating_sub(token.len()) + 1;
                            token.push_str(
                                &line_text
                                    [(offset + 1).min(line_len)..(offset + end + 1).min(line_len)],
                            );

                            if let Some(node) = match_color(&token, ix, token_offset) {
                                token.clear();
                                nodes.push(node);
                                offset += end + 1;
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {
                    token.clear();
                }
            }

            offset += 1;
        }
    }

    nodes
}

fn match_color(part: &str, line_ix: usize, offset: usize) -> Option<ColorNode> {
    if let Ok(color) = try_parse_color(part) {
        Some(ColorNode::new(part, color, line_ix + 1, offset + 1))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use csscolorparser::Color;

    use crate::parser::{match_color, parse, try_parse_gpui_color, ColorNode};

    #[test]
    fn test_match_color() {
        let cases = vec![
            "#A0F0F0",
            "#2eC8f1",
            "#AAF0F0aa",
            "#AAF0F033",
            "#0f0E",
            "#F2c",
            "rgb(80%,80%,20%)",
            "rgb(255 100 0)",
            "rgba(255, 0, 0, 0.5)",
            "rgb(100, 200, 100)",
            "hsl(225, 100%, 70%)",
            "hsla(20, 100%, 50%, .5)",
            "hsla(1., 0.5, 0.5, 1.)",
        ];

        for case in cases {
            assert!(match_color(case, 1, 1).is_some());
        }

        assert_eq!(
            match_color("#e7b911", 1, 10),
            Some(ColorNode::must_parse("#e7b911", 2, 11))
        );
    }

    #[test]
    fn test_try_parse_gpui_color() {
        assert_eq!(
            try_parse_gpui_color("rgb(0., 1., 0.2)"),
            Ok(Color {
                r: 0.,
                g: 1.,
                b: 0.2,
                a: 1.
            })
        );
        assert_eq!(
            try_parse_gpui_color("rgb(0., 1., 0., 0.45)"),
            Ok(Color {
                r: 0.,
                g: 1.,
                b: 0.,
                a: 0.45
            })
        );
        assert!(try_parse_gpui_color("rgb(255., 220.0, 0.)").is_err());
        assert!(try_parse_gpui_color("rgba(255., 120., 20.0, 1.)").is_err());

        assert_eq!(
            try_parse_gpui_color("hsl(0.48, 1., 0.45)"),
            Ok(Color::new(0., 0.9, 0.79200006, 1.))
        );
        assert_eq!(
            try_parse_gpui_color("hsla(0.48, 1., 0.45, 0.3)"),
            Ok(Color::new(0., 0.9, 0.79200006, 0.3))
        );
        assert!(try_parse_gpui_color("hsl(240., 0., 50.0)").is_err());
        assert!(try_parse_gpui_color("hsla(240., 0., 50.0, 1.)").is_err());
    }

    #[test]
    fn test_must_parse() {
        assert_eq!(
            ColorNode::must_parse("hsla(.2, 0.5, 0.5, 1.)", 10, 12),
            ColorNode {
                matched: "hsla(.2, 0.5, 0.5, 1.)".to_string(),
                color: Color::from_hsla(0.2 * 360., 0.5, 0.5, 1.),
                loc: (10, 12)
            }
        );

        assert_eq!(
            ColorNode::must_parse("rgba(1., 0.5, 0.5, 1.)", 10, 12),
            ColorNode {
                matched: "rgba(1., 0.5, 0.5, 1.)".to_string(),
                color: Color::new(1., 0.5, 0.5, 1.),
                loc: (10, 12)
            }
        );
    }

    #[test]
    fn test_parse() {
        let colors = parse(include_str!("../../tests/test.json"));

        assert_eq!(colors.len(), 8);
        assert_eq!(colors[0], ColorNode::must_parse("#999", 2, 15));
        assert_eq!(colors[1], ColorNode::must_parse("#FFFFFF", 3, 18));
        assert_eq!(colors[2], ColorNode::must_parse("#ff003c99", 4, 13));
        assert_eq!(colors[3], ColorNode::must_parse("#3cBD00", 5, 15));
        assert_eq!(
            colors[4],
            ColorNode::must_parse("rgba(255, 252, 0, 0.5)", 6, 12)
        );
        assert_eq!(
            colors[5],
            ColorNode::must_parse("rgb(100, 200, 100)", 7, 11)
        );
        assert_eq!(
            colors[6],
            ColorNode::must_parse("hsla(20, 100%, 50%, .5)", 8, 12)
        );
        assert_eq!(
            colors[7],
            ColorNode::must_parse("hsl(225, 100%, 70%)", 9, 11)
        );
    }
}
