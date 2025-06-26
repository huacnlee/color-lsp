use csscolorparser::Color;
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
        let color =
            csscolorparser::parse(matched).expect("The `matched` should be a valid CSS color");
        Self {
            matched: matched.to_string(),
            loc: (line, col),
            color,
        }
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
            let c = line_text.chars().nth(offset).unwrap();
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
    if let Ok(color) = csscolorparser::parse(part) {
        Some(ColorNode::new(part, color, line_ix + 1, offset + 1))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{match_color, parse, ColorNode};

    #[test]
    fn test_match_color() {
        let cases = vec![
            "#999999",
            "#c0c0c0",
            "#F0F0F0aa",
            "#EfEfEfcc",
            "#F0f",
            "rgb(100%,0%,0%)",
            "rgb(255 0 0)",
            "rgba(255, 0, 0, 0.5)",
            "rgb(255, 0, 0)",
            "hsl(0, 100%, 50%)",
        ];

        for case in cases {
            assert!(match_color(case, 1, 1).is_some());
        }

        assert_eq!(
            match_color("#ff22ff", 1, 10),
            Some(ColorNode::must_parse("#ff22ff", 2, 11))
        );
    }

    #[test]
    fn test_parse() {
        let colors = parse(include_str!("../../tests/test.json"));

        assert_eq!(colors.len(), 8);
        assert_eq!(colors[0], ColorNode::must_parse("#999", 2, 15));
        assert_eq!(colors[1], ColorNode::must_parse("#FFFFFF", 3, 18));
        assert_eq!(colors[2], ColorNode::must_parse("#ff000099", 4, 13));
        assert_eq!(colors[3], ColorNode::must_parse("#00FF00", 5, 15));
        assert_eq!(
            colors[4],
            ColorNode::must_parse("rgba(255, 0, 0, 0.5)", 6, 12)
        );
        assert_eq!(colors[5], ColorNode::must_parse("rgb(255, 0, 0)", 7, 11));
        assert_eq!(
            colors[6],
            ColorNode::must_parse("hsla(0, 100%, 50%, 0.5)", 8, 12)
        );
        assert_eq!(colors[7], ColorNode::must_parse("hsl(0, 100%, 50%)", 9, 11));
    }
}
