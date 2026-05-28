use roxmltree::Document;
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpmlRow {
    pub title: Option<String>,
    pub feed_url: String,
    pub html_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpmlImportOutcome {
    pub status: String,
    pub feed_url: Option<String>,
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpmlImportResult {
    pub rows: Vec<OpmlImportOutcome>,
    pub added: usize,
    pub skipped_duplicate: usize,
    pub rejected_scheme: usize,
    pub malformed: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpmlImportError {
    message: String,
}

impl OpmlImportError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for OpmlImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for OpmlImportError {}

pub fn parse_opml(source: &[u8]) -> Result<Vec<OpmlRow>, OpmlImportError> {
    let text = std::str::from_utf8(source)
        .map_err(|exc| OpmlImportError::new(format!("OPML file is not UTF-8: {exc}")))?;
    if text.contains("<!ENTITY") {
        return Err(OpmlImportError::new(
            "OPML file contains external entity declarations (XXE); rejected for safety.",
        ));
    }
    let doc = Document::parse(text)
        .map_err(|exc| OpmlImportError::new(format!("OPML file could not be parsed: {exc}")))?;
    let root = doc.root_element();
    if root.tag_name().name().to_ascii_lowercase() != "opml" {
        return Err(OpmlImportError::new("OPML root is not an <opml> element"));
    }

    let mut rows = Vec::new();
    for node in doc
        .descendants()
        .filter(|node| node.has_tag_name("outline"))
    {
        let xml_url = node.attribute("xmlUrl").unwrap_or_default().trim();
        if xml_url.is_empty() {
            if matches!(
                node.attribute("type")
                    .map(str::to_ascii_lowercase)
                    .as_deref(),
                Some("rss" | "atom")
            ) {
                rows.push(OpmlRow {
                    title: attr_string(node.attribute("text")),
                    feed_url: String::new(),
                    html_url: attr_string(node.attribute("htmlUrl")),
                });
            }
            continue;
        }
        rows.push(OpmlRow {
            title: attr_string(node.attribute("text"))
                .or_else(|| attr_string(node.attribute("title"))),
            feed_url: xml_url.to_string(),
            html_url: attr_string(node.attribute("htmlUrl")),
        });
    }
    Ok(rows)
}

pub fn validate_opml_rows(
    rows: Vec<OpmlRow>,
    existing_feed_urls: HashSet<String>,
) -> OpmlImportResult {
    let outcomes: Vec<OpmlImportOutcome> = rows
        .into_iter()
        .map(|row| {
            if row.feed_url.is_empty() {
                return OpmlImportOutcome {
                    status: "malformed".to_string(),
                    feed_url: None,
                    title: row.title,
                    message: Some("Missing xmlUrl attribute".to_string()),
                };
            }
            if !is_http_url(&row.feed_url) {
                return OpmlImportOutcome {
                    status: "rejected_scheme".to_string(),
                    feed_url: Some(row.feed_url.clone()),
                    title: row.title,
                    message: Some(format!("Unsupported URL scheme: {:?}", row.feed_url)),
                };
            }
            if existing_feed_urls.contains(&row.feed_url) {
                return OpmlImportOutcome {
                    status: "skipped_duplicate".to_string(),
                    feed_url: Some(row.feed_url),
                    title: row.title,
                    message: None,
                };
            }
            OpmlImportOutcome {
                status: "added".to_string(),
                feed_url: Some(row.feed_url),
                title: row.title,
                message: None,
            }
        })
        .collect();
    OpmlImportResult {
        added: outcomes.iter().filter(|row| row.status == "added").count(),
        skipped_duplicate: outcomes
            .iter()
            .filter(|row| row.status == "skipped_duplicate")
            .count(),
        rejected_scheme: outcomes
            .iter()
            .filter(|row| row.status == "rejected_scheme")
            .count(),
        malformed: outcomes
            .iter()
            .filter(|row| row.status == "malformed")
            .count(),
        rows: outcomes,
    }
}

pub fn import_opml(
    source: &[u8],
    existing_feed_urls: HashSet<String>,
) -> Result<OpmlImportResult, OpmlImportError> {
    Ok(validate_opml_rows(parse_opml(source)?, existing_feed_urls))
}

pub fn render_opml(subscriptions: Vec<(String, String, Option<String>)>) -> Vec<u8> {
    let mut output = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<opml version=\"2.0\">\n  <head>\n    <title>Station Scout subscriptions</title>\n    <docs>https://github.com/Orinks/station-scout</docs>\n  </head>\n  <body>\n",
    );
    for (title, feed_url, html_url) in subscriptions {
        output.push_str("    <outline type=\"rss\"");
        if !title.is_empty() {
            output.push_str(&format!(" text=\"{}\"", escape_xml(&title)));
        }
        output.push_str(&format!(" xmlUrl=\"{}\"", escape_xml(&feed_url)));
        if let Some(html_url) = html_url.filter(|value| !value.is_empty()) {
            output.push_str(&format!(" htmlUrl=\"{}\"", escape_xml(&html_url)));
        }
        output.push_str(" />\n");
    }
    output.push_str("  </body>\n</opml>");
    output.into_bytes()
}

fn attr_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn is_http_url(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
