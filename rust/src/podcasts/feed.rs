use regex::Regex;
use roxmltree::{Document, Node};
use sha1::{Digest, Sha1};
use std::fmt;
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transcript {
    pub url: String,
    pub mime_type: Option<String>,
    pub language: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Episode {
    pub guid: Option<String>,
    pub title: Option<String>,
    pub enclosure_url: Option<String>,
    pub enclosure_url_norm: Option<String>,
    pub title_pubdate_hash: Option<String>,
    pub pub_date: Option<String>,
    pub duration_seconds: Option<i32>,
    pub description_html: Option<String>,
    pub transcript: Option<Transcript>,
    pub chapters_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedFeed {
    pub feed_title: Option<String>,
    pub feed_author: Option<String>,
    pub feed_image_url: Option<String>,
    pub feed_description: Option<String>,
    pub feed_link: Option<String>,
    pub episodes: Vec<Episode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedParseError {
    message: String,
}

impl FeedParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for FeedParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FeedParseError {}

pub fn parse_feed(source: &[u8]) -> Result<ParsedFeed, FeedParseError> {
    let text = std::str::from_utf8(source)
        .map_err(|exc| FeedParseError::new(format!("Feed is not UTF-8: {exc}")))?;
    let doc = Document::parse(text)
        .map_err(|exc| FeedParseError::new(format!("Feed could not be parsed: {exc}")))?;
    let channel = doc
        .descendants()
        .find(|node| node.has_tag_name("channel"))
        .unwrap_or_else(|| doc.root_element());

    let episodes = channel
        .children()
        .filter(|node| node.has_tag_name("item") || node.has_tag_name("entry"))
        .map(parse_episode)
        .collect();

    Ok(ParsedFeed {
        feed_title: child_text(channel, "title"),
        feed_author: child_text(channel, "author").or_else(|| child_text(channel, "itunes:author")),
        feed_image_url: image_url(channel),
        feed_description: child_text(channel, "subtitle")
            .or_else(|| child_text(channel, "description"))
            .or_else(|| child_text(channel, "summary")),
        feed_link: child_text(channel, "link"),
        episodes,
    })
}

pub fn normalize_enclosure_url(url: &str) -> Option<String> {
    let mut cleaned = url.trim().to_string();
    if cleaned.is_empty() {
        return None;
    }
    for _ in 0..4 {
        let stripped = strip_one_prefix(&cleaned);
        if stripped == cleaned {
            break;
        }
        cleaned = stripped;
    }
    Some(cleaned)
}

pub fn compute_title_pubdate_hash(title: &str, pub_date: &str) -> Option<String> {
    if title.is_empty() || pub_date.is_empty() {
        return None;
    }
    let mut hasher = Sha1::new();
    hasher.update(format!("{title}|{pub_date}").as_bytes());
    Some(to_lower_hex(&hasher.finalize()))
}

fn parse_episode(node: Node<'_, '_>) -> Episode {
    let guid = child_text(node, "guid").or_else(|| child_text(node, "id"));
    let title = child_text(node, "title");
    let enclosure_url = first_enclosure(node);
    let pub_date = child_text(node, "pubDate")
        .or_else(|| child_text(node, "published"))
        .or_else(|| child_text(node, "updated"));
    let duration_seconds = child_text(node, "itunes:duration")
        .or_else(|| child_text(node, "duration"))
        .and_then(|value| parse_duration_seconds(&value));
    let description_html = child_text(node, "description")
        .or_else(|| child_text(node, "summary"))
        .or_else(|| child_text(node, "content"));
    let transcript = transcript(node);
    let chapters_url = node
        .children()
        .find(|child| child.tag_name().name() == "chapters")
        .and_then(|child| attr_string(child.attribute("url").or_else(|| child.attribute("href"))));
    let enclosure_url_norm = enclosure_url.as_deref().and_then(normalize_enclosure_url);
    let title_pubdate_hash = title
        .as_deref()
        .zip(pub_date.as_deref())
        .and_then(|(title, pub_date)| compute_title_pubdate_hash(title, pub_date));

    Episode {
        guid,
        title,
        enclosure_url,
        enclosure_url_norm,
        title_pubdate_hash,
        pub_date,
        duration_seconds,
        description_html,
        transcript,
        chapters_url,
    }
}

fn child_text(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.children()
        .find(|child| tag_matches(*child, name))
        .and_then(|child| child.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn tag_matches(node: Node<'_, '_>, expected: &str) -> bool {
    if !node.is_element() {
        return false;
    }
    node.tag_name().name() == expected
        || expected
            .split_once(':')
            .is_some_and(|(_, local)| node.tag_name().name() == local)
}

fn image_url(node: Node<'_, '_>) -> Option<String> {
    node.children()
        .find(|child| child.has_tag_name("image"))
        .and_then(|image| child_text(image, "url").or_else(|| attr_string(image.attribute("href"))))
        .or_else(|| {
            node.children()
                .find(|child| child.tag_name().name() == "image")
                .and_then(|image| attr_string(image.attribute("href")))
        })
}

fn first_enclosure(node: Node<'_, '_>) -> Option<String> {
    node.children()
        .find(|child| child.has_tag_name("enclosure"))
        .and_then(|child| attr_string(child.attribute("url").or_else(|| child.attribute("href"))))
        .or_else(|| {
            node.children()
                .find(|child| {
                    child.has_tag_name("link") && child.attribute("rel") == Some("enclosure")
                })
                .and_then(|child| attr_string(child.attribute("href")))
        })
}

fn parse_duration_seconds(value: &str) -> Option<i32> {
    let text = value.trim();
    if let Ok(seconds) = text.parse::<i32>() {
        return Some(seconds);
    }
    let parts: Vec<&str> = text.split(':').collect();
    match parts.as_slice() {
        [minutes, seconds] => {
            Some(minutes.parse::<i32>().ok()? * 60 + seconds.parse::<i32>().ok()?)
        }
        [hours, minutes, seconds] => Some(
            hours.parse::<i32>().ok()? * 3600
                + minutes.parse::<i32>().ok()? * 60
                + seconds.parse::<i32>().ok()?,
        ),
        _ => None,
    }
}

fn transcript(node: Node<'_, '_>) -> Option<Transcript> {
    node.children()
        .find(|child| child.tag_name().name() == "transcript")
        .and_then(|child| {
            Some(Transcript {
                url: attr_string(child.attribute("url").or_else(|| child.attribute("href")))?,
                mime_type: attr_string(
                    child
                        .attribute("type")
                        .or_else(|| child.attribute("mime_type")),
                ),
                language: attr_string(child.attribute("language")),
            })
        })
}

fn strip_one_prefix(value: &str) -> String {
    let Ok(parsed) = Url::parse(value) else {
        return value.to_string();
    };
    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    let prefix_hosts = [
        "chartable.com",
        "op3.dev",
        "pdst.fm",
        "podtrac.com",
        "www.podtrac.com",
        "dts.podtrac.com",
        "chtbl.com",
        "claritaspod.com",
    ];
    if !prefix_hosts.contains(&host.as_str()) {
        return value.to_string();
    }
    let path = parsed.path();
    if let Some(index) = path.find("http://").or_else(|| path.find("https://")) {
        return path[index..].to_string();
    }

    let podtrac_re = Regex::new(r"^/[^/]+\.(?:mp3|m4a|mp4|ogg|opus|aac|wav)/(?P<rest>.+)$")
        .expect("valid regex");
    if let Some(captures) = podtrac_re.captures(path) {
        let rest = captures
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or_default();
        if Regex::new(r"^[a-z0-9][a-z0-9.\-]*\.[a-z]{2,}(?:/|$)")
            .expect("valid regex")
            .is_match(rest)
        {
            return format!("https://{rest}");
        }
    }
    value.to_string()
}

fn attr_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
