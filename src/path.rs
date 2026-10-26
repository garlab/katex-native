//! KaTeX's SVG path data, turned into absolute drawing commands.

/// One step of a filled outline, in absolute coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(f64, f64),
    LineTo(f64, f64),
    /// A cubic Bézier: two control points, then the end point.
    CubicTo(f64, f64, f64, f64, f64, f64),
    Close,
}

/// Parses SVG path data with the commands KaTeX uses (M, L, H, V, C, S, Z
/// and their relative forms), then maps each point through `transform`.
/// Parsing stops at anything else, keeping the commands read so far.
pub(crate) fn parse(data: &str, transform: impl Fn(f64, f64) -> (f64, f64)) -> Vec<PathCommand> {
    let mut commands = Vec::new();
    let mut tokens = Tokens { rest: data };
    let (mut x, mut y) = (0., 0.);
    let (mut start_x, mut start_y) = (0., 0.);
    // The second control point of the last cubic, for S.
    let mut last_control: Option<(f64, f64)> = None;
    let mut command: Option<char> = None;
    while let Some(token) = tokens.peek() {
        let letter = match token {
            Token::Command(letter) => {
                tokens.next();
                letter
            }
            // Extra numbers repeat the previous command; after M they mean L.
            Token::Number(_) => match command {
                Some('M') => 'L',
                Some('m') => 'l',
                Some(letter) if !letter.eq_ignore_ascii_case(&'z') => letter,
                _ => break,
            },
        };
        command = Some(letter);
        let relative = letter.is_ascii_lowercase();
        let (dx, dy) = if relative { (x, y) } else { (0., 0.) };
        let upper = letter.to_ascii_uppercase();
        if upper == 'Z' {
            commands.push(PathCommand::Close);
            (x, y) = (start_x, start_y);
            last_control = None;
            continue;
        }
        let count = match upper {
            'M' | 'L' => 2,
            'H' | 'V' => 1,
            'S' => 4,
            'C' => 6,
            _ => break,
        };
        let Some(p) = tokens.numbers(count) else {
            break;
        };
        match upper {
            'M' => {
                (x, y) = (p[0] + dx, p[1] + dy);
                (start_x, start_y) = (x, y);
                let (tx, ty) = transform(x, y);
                commands.push(PathCommand::MoveTo(tx, ty));
                last_control = None;
            }
            'L' | 'H' | 'V' => {
                match upper {
                    'L' => (x, y) = (p[0] + dx, p[1] + dy),
                    'H' => x = p[0] + dx,
                    _ => y = p[0] + dy,
                }
                let (tx, ty) = transform(x, y);
                commands.push(PathCommand::LineTo(tx, ty));
                last_control = None;
            }
            _ => {
                let (c1, rest) = if upper == 'S' {
                    // Reflect the previous second control point about the
                    // current point.
                    let c1 = last_control.map_or((x, y), |(cx, cy)| (2. * x - cx, 2. * y - cy));
                    (c1, &p[..])
                } else {
                    ((p[0] + dx, p[1] + dy), &p[2..])
                };
                let c2 = (rest[0] + dx, rest[1] + dy);
                (x, y) = (rest[2] + dx, rest[3] + dy);
                let (a, b) = transform(c1.0, c1.1);
                let (c, d) = transform(c2.0, c2.1);
                let (e, f) = transform(x, y);
                commands.push(PathCommand::CubicTo(a, b, c, d, e, f));
                last_control = Some(c2);
            }
        }
    }
    commands
}

#[derive(Clone, Copy)]
enum Token {
    Command(char),
    Number(f64),
}

#[derive(Clone, Copy)]
struct Tokens<'a> {
    rest: &'a str,
}

impl Tokens<'_> {
    fn peek(&self) -> Option<Token> {
        let mut copy = *self;
        copy.next()
    }

    fn numbers(&mut self, count: usize) -> Option<Vec<f64>> {
        (0..count)
            .map(|_| match self.next()? {
                Token::Number(n) => Some(n),
                Token::Command(_) => None,
            })
            .collect()
    }

    fn next(&mut self) -> Option<Token> {
        self.rest = self
            .rest
            .trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        let first = self.rest.chars().next()?;
        if first.is_ascii_alphabetic() {
            self.rest = &self.rest[1..];
            return Some(Token::Command(first));
        }
        // A number: optional sign, digits with at most one dot, optional
        // exponent. A second dot or a sign starts the next number.
        let bytes = self.rest.as_bytes();
        let mut end = usize::from(matches!(bytes.first(), Some(b'-' | b'+')));
        let mut seen_dot = false;
        while let Some(&b) = bytes.get(end) {
            match b {
                b'0'..=b'9' => end += 1,
                b'.' if !seen_dot => {
                    seen_dot = true;
                    end += 1;
                }
                b'e' | b'E' if matches!(bytes.get(end + 1), Some(b'0'..=b'9' | b'-' | b'+')) => {
                    end += 2;
                }
                _ => break,
            }
        }
        let number = self.rest[..end].parse().ok()?;
        self.rest = &self.rest[end..];
        Some(Token::Number(number))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_relative_and_smooth_commands() {
        let commands = parse("M10 20h5v-5c1 2 3 4 5 6s7 8 9 10l1 1z", |x, y| (x, y));
        assert_eq!(
            commands,
            [
                PathCommand::MoveTo(10., 20.),
                PathCommand::LineTo(15., 20.),
                PathCommand::LineTo(15., 15.),
                PathCommand::CubicTo(16., 17., 18., 19., 20., 21.),
                // The first control point reflects (18, 19) about (20, 21).
                PathCommand::CubicTo(22., 23., 27., 29., 29., 31.),
                PathCommand::LineTo(30., 32.),
                PathCommand::Close,
            ]
        );
    }

    #[test]
    fn repeats_commands_for_extra_numbers() {
        let commands = parse("M0 0 10 0 10 10H0z", |x, y| (x, y));
        assert_eq!(
            commands,
            [
                PathCommand::MoveTo(0., 0.),
                PathCommand::LineTo(10., 0.),
                PathCommand::LineTo(10., 10.),
                PathCommand::LineTo(0., 10.),
                PathCommand::Close,
            ]
        );
    }

    #[test]
    fn splits_numbers_without_separators() {
        let commands = parse("M1.5.5l-1-1", |x, y| (x, y));
        assert_eq!(
            commands,
            [
                PathCommand::MoveTo(1.5, 0.5),
                PathCommand::LineTo(0.5, -0.5)
            ]
        );
    }
}
