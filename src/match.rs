use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
};

use anyhow::{anyhow, Result};
use crossterm::style::{self, Attribute, Color, Stylize};

#[derive(Debug, PartialEq, Eq)]
pub struct Matches {
    text: String,
    matches: Vec<Match>,
    is_replaced: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Match {
    points: Vec<usize>,
}

impl Matches {
    pub fn new(text: impl Into<String>, matcher: &str, wildcard: &str) -> Result<Self> {
        let text = text.into();

        if matcher.is_empty() {
            return Err(anyhow!("matcher cannot be empty"));
        }

        let matcher = matcher.split(wildcard).collect::<Vec<_>>();
        if matcher.iter().any(|x| x.is_empty()) {
            return Err(anyhow!(
                "wildcards have to be enclosed in non-empty strings"
            ));
        }
        let mut i = 0;
        let mut matches = Vec::new();

        while i < text.len() {
            if let Some(m) = Self::try_match(&text, &matcher, i) {
                i = *m.points.last().unwrap();
                matches.push(m);
            } else {
                break;
            }
        }

        Ok(Self {
            text,
            matches,
            is_replaced: false,
        })
    }

    fn try_match(text: &str, matcher: &[&str], mut i: usize) -> Option<Match> {
        let mut points = Vec::with_capacity(matcher.len() * 2);

        for &matcher in matcher {
            if let Some(index) = text[i..].find(matcher) {
                let begin = i + index;
                let end = begin + matcher.len();
                points.push(begin);
                points.push(end);
                i = end;
            } else {
                return None;
            }
        }

        Some(Match { points })
    }

    pub fn replace(
        &self,
        replace: &str,
        wildcard: &str,
        reorder: impl IntoIterator<Item = usize> + Clone,
    ) -> Result<Self> {
        let replacer = replace.split(wildcard).collect::<Vec<_>>();
        let mut result = String::new();
        let mut i = 0;
        let mut new_matches = Vec::with_capacity(self.matches.len());

        for m in &self.matches {
            let mut new_match = Vec::with_capacity(m.points.len());
            let mut reorder = reorder.clone().into_iter();
            let mut seen = BTreeSet::new();
            result.push_str(&self.text[i..m.points[0]]);
            new_match.push(result.len());

            for (i, &r) in replacer.iter().enumerate() {
                result.push_str(r);
                new_match.push(result.len());
                if i == replacer.len() - 1 {
                    break;
                }
                let index = if let Some(index) = reorder.next() {
                    index
                } else {
                    'index: {
                        let mut last = 0;
                        for &v in &seen {
                            if v > last {
                                break 'index last;
                            }
                            last = v + 1;
                        }
                        seen.last().copied().map(|x| x + 1).unwrap_or(0)
                    }
                };
                seen.insert(index);
                let (Some(&begin), Some(&end)) =
                    (m.points.get(index * 2 + 1), m.points.get(index * 2 + 2))
                else {
                    return Err(anyhow!("not enough wildcard matches for replace pattern"));
                };
                result.push_str(&self.text[begin..end]);
                new_match.push(result.len());
            }

            new_matches.push(Match { points: new_match });
            i = *m.points.last().unwrap();
        }

        result.push_str(&self.text[i..]);
        Ok(Matches {
            text: result,
            matches: new_matches,
            is_replaced: true,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

impl Display for Matches {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let line_breaks = get_line_breaks(&self.text);
        for (n, m) in self.matches.iter().enumerate() {
            let begin = m.points[0];
            let end = m.points[m.points.len() - 1];
            let begin_line = get_line_number(&line_breaks, begin);
            let end_line = get_line_number(&line_breaks, end);
            let begin_col = begin - line_breaks[begin_line];
            let end_col = end - line_breaks[end_line];
            if begin_line == end_line {
                writeln!(
                    f,
                    "Ln {} from Col {} to Col {}:",
                    begin_line + 1,
                    begin_col + 1,
                    end_col
                )?;
            } else {
                writeln!(
                    f,
                    "from (Ln {}, Col {}) to (Ln {}, Col {}):",
                    begin_line + 1,
                    begin_col + 1,
                    end_line + 1,
                    end_col
                )?;
            }

            let max_line_width = ((end_line + 1) as f64).log10() as usize + 1;

            let mut points = m.points.iter().copied().peekable();
            let mut status = 0;
            for line in begin_line..=end_line {
                write!(
                    f,
                    "{}{:>max_line_width$}{} ",
                    style::SetForegroundColor(Color::Yellow),
                    line + 1,
                    style::SetForegroundColor(Color::Reset),
                )?;

                let mut current = line_breaks[line];
                let line_end = line_breaks
                    .get(line + 1)
                    .copied()
                    .unwrap_or(self.text.len());
                let mut first_loop = true;
                while current < line_end {
                    let end = match points.peek().copied() {
                        Some(end) => {
                            if first_loop {
                                first_loop = false;
                            } else {
                                status = match status {
                                    0 => 1,
                                    1 => 2,
                                    _ => 1,
                                };
                            }
                            if end < line_end {
                                points.next();
                                end
                            } else {
                                line_end
                            }
                        }
                        None => {
                            status = 0;
                            line_end
                        }
                    };

                    let content = &self.text[current..end].trim_end_matches(['\n', '\r']);

                    let content = match status {
                        1 => {
                            if self.is_replaced {
                                content.green()
                            } else {
                                content.red()
                            }
                        }
                        2 => content.magenta().underlined(),
                        _ => content.attribute(Attribute::Dim),
                    };

                    write!(f, "{}", content)?;
                    current = end;
                }
                writeln!(f)?;
            }
            if n < self.matches.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

fn get_line_breaks(text: &str) -> Vec<usize> {
    let mut breaks = Vec::new();
    for line in text.lines() {
        let offset = line.as_ptr() as usize - text.as_ptr() as usize;
        breaks.push(offset);
    }
    breaks
}

fn get_line_number(line_breaks: &[usize], index: usize) -> usize {
    match line_breaks.binary_search(&index) {
        Ok(n) => n,
        Err(n) => n - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document() {
        let text = "hello world, hero!";
        let matcher = "he*o";
        let wildcard = "*";
        let document = Matches::new(text, matcher, wildcard).unwrap();
        assert_eq!(
            document.matches,
            vec![
                Match {
                    points: vec![0, 2, 4, 5]
                },
                Match {
                    points: vec![13, 15, 16, 17]
                }
            ]
        );
    }

    #[test]
    fn test_get_line_breaks() {
        let text = "hello\nworld\r\n, hero!";
        let line_breaks = get_line_breaks(text);
        assert_eq!(line_breaks, vec![0, 6, 13]);
        assert_eq!(get_line_number(&line_breaks, 0), 0);
        assert_eq!(get_line_number(&line_breaks, 5), 0);
        assert_eq!(get_line_number(&line_breaks, 6), 1);
        assert_eq!(get_line_number(&line_breaks, 12), 1);
        assert_eq!(get_line_number(&line_breaks, 13), 2);
    }
}
