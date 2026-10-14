//! Links in text someone pasted or imported (a list, an email, a web page),
//! for adding several downloads at once (docs/decisions/0036).

use std::collections::HashSet;

use reqwest::Url;

use crate::category::Matched;
use crate::download::check_scheme;
use crate::error::Error;

/// Something in the text that looks like a link (`scheme://…`).
#[derive(Debug)]
pub enum Found {
    Link(Url),
    /// It can't be downloaded: `error` is [`Error::InvalidUrl`] or
    /// [`Error::UnsupportedScheme`].
    Invalid {
        text: String,
        error: Error,
    },
}

/// A link found, and what adding it would do.
#[derive(Debug)]
pub enum LinkPreview {
    Link {
        url: Url,
        /// The file name the link suggests; the server may give another.
        name: String,
        /// The category it would get without one chosen.
        matched: Matched,
        /// A download of this same link is already on the list.
        known: bool,
    },
    Invalid {
        text: String,
        error: Error,
    },
}

/// Every link in `text`, in order, each once: anything of the form
/// `scheme://…` up to a space, a quote or an angle bracket, without the
/// punctuation that ends a sentence around it.
pub fn find(text: &str) -> Vec<Found> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut rest = 0;
    while let Some(at) = text[rest..].find("://") {
        let separator = rest + at;
        // The scheme is the letters just before.
        let start = text[..separator]
            .trim_end_matches(|c: char| c.is_ascii_alphabetic())
            .len();
        // "://" is no boundary, so this is past it.
        let end = text[separator..]
            .find(is_boundary)
            .map_or(text.len(), |len| separator + len);
        rest = end;
        if start == separator {
            continue; // No scheme.
        }
        let candidate = trim_end(&text[start..end]);
        let one = match Url::parse(candidate) {
            Ok(url) => match check_scheme(&url) {
                Ok(()) => Found::Link(url),
                Err(error) => invalid(candidate, error),
            },
            Err(_) => invalid(candidate, Error::InvalidUrl),
        };
        let key = match &one {
            Found::Link(url) => url.as_str(),
            Found::Invalid { text, .. } => text.as_str(),
        };
        if seen.insert(key.to_owned()) {
            found.push(one);
        }
    }
    found
}

fn invalid(text: &str, error: Error) -> Found {
    Found::Invalid {
        text: text.to_owned(),
        error,
    }
}

/// Ends a link: links can't hold these as they are (they're percent-encoded).
fn is_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '<' | '>' | '`')
}

/// Without what likely belongs to the sentence around it: a final full stop,
/// comma or quote, or a closing bracket the link doesn't open itself (as in
/// "(see https://example.com/a)", but not "https://en.wikipedia.org/wiki/Otter_(disambiguation)").
fn trim_end(link: &str) -> &str {
    let mut link = link;
    while let Some(last) = link.chars().next_back() {
        let open = match last {
            ')' => Some('('),
            ']' => Some('['),
            '}' => Some('{'),
            _ => None,
        };
        let trailing = match open {
            Some(open) => link.matches(open).count() < link.matches(last).count(),
            None => matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | '\''),
        };
        if !trailing {
            break;
        }
        link = &link[..link.len() - last.len_utf8()];
    }
    link
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The links found, as text; invalid ones as "invalid: …".
    fn found(text: &str) -> Vec<String> {
        find(text)
            .into_iter()
            .map(|found| match found {
                Found::Link(url) => url.to_string(),
                Found::Invalid { text, error } => format!("invalid: {text} ({error})"),
            })
            .collect()
    }

    #[test]
    fn one_link_per_line() {
        let text = "https://example.com/a.zip\r\nhttp://example.com/b.iso\n\n  https://example.com/c d.mp4\n";
        assert_eq!(
            found(text),
            [
                "https://example.com/a.zip",
                "http://example.com/b.iso",
                "https://example.com/c",
            ]
        );
    }

    #[test]
    fn links_in_prose_lose_the_punctuation_around_them() {
        let text = "Get https://example.com/a.zip, then (see https://example.com/b.zip). \
                    Also \"https://example.com/c.zip\" and <https://example.com/d.zip>! \
                    Or [this](https://example.com/e.zip): 'https://example.com/f.zip'.";
        assert_eq!(
            found(text),
            [
                "https://example.com/a.zip",
                "https://example.com/b.zip",
                "https://example.com/c.zip",
                "https://example.com/d.zip",
                "https://example.com/e.zip",
                "https://example.com/f.zip",
            ]
        );
    }

    #[test]
    fn brackets_the_link_opens_stay() {
        assert_eq!(
            found("(https://en.wikipedia.org/wiki/Otter_(disambiguation))"),
            ["https://en.wikipedia.org/wiki/Otter_(disambiguation)"]
        );
        assert_eq!(
            found("http://[::1]:8080/file.bin"),
            ["http://[::1]:8080/file.bin"]
        );
    }

    #[test]
    fn links_in_html() {
        let text =
            r#"<a href="https://example.com/a.zip">A</a><img src="https://example.com/b.png?w=2">"#;
        assert_eq!(
            found(text),
            ["https://example.com/a.zip", "https://example.com/b.png?w=2"]
        );
    }

    #[test]
    fn each_link_once_the_first_time() {
        let text = "https://example.com/a.zip HTTPS://EXAMPLE.COM/a.zip https://example.com/b.zip https://example.com/a.zip";
        assert_eq!(
            found(text),
            ["https://example.com/a.zip", "https://example.com/b.zip"]
        );
    }

    #[test]
    fn links_that_cant_be_downloaded_are_reported() {
        assert_eq!(
            found("ftp://example.com/a.zip http://exa%mple.com/ ://nothing https://"),
            [
                "invalid: ftp://example.com/a.zip (unsupported URL scheme `ftp` (only http and https are supported))",
                "invalid: http://exa%mple.com/ (not a valid link)",
                "invalid: https:// (not a valid link)",
            ]
        );
    }

    #[test]
    fn the_scheme_is_the_letters_just_before() {
        assert_eq!(
            found("Link:https://example.com/a.zip é.http://example.com/b.zip"),
            ["https://example.com/a.zip", "http://example.com/b.zip"]
        );
    }

    #[test]
    fn no_links() {
        assert!(found("").is_empty());
        assert!(found("nothing to see, not even example.com/a.zip").is_empty());
    }
}
