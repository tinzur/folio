//! Configurable web actions for selected text (Preferences ▸ Search the web).
//!
//! Each action is a label and a URL template with a `{query}` placeholder. Selecting text and
//! choosing the action opens the template in the system browser, with the selected words (percent
//! encoded) in place of the placeholder. Search engines and AI tools need no built-in chatbot: a
//! URL is enough, and the list is local to the user's settings.

use serde::{Deserialize, Serialize};

/// The placeholder in a URL template that the selected text replaces.
pub const QUERY: &str = "{query}";

/// The most actions the list keeps (also the cap `restore` accepts).
pub const MAX_TOOLS: usize = 20;

/// One web action offered for selected text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebTool {
    /// The name shown in the menu ("Google", "ChatGPT", …).
    pub label: String,
    /// The URL to open, with [`QUERY`] where the selected text goes.
    pub url: String,
}

impl WebTool {
    pub fn new(label: impl Into<String>, url: impl Into<String>) -> Self {
        Self { label: label.into(), url: url.into() }
    }

    /// Whether this action can be offered: a name and a template with the placeholder.
    pub fn is_valid(&self) -> bool {
        !self.label.trim().is_empty() && self.url.contains(QUERY)
    }

    /// The URL to open for `query`, with the words percent encoded. `None` for an invalid
    /// template, so a half-written entry is never opened.
    pub fn url_for(&self, query: &str) -> Option<String> {
        self.is_valid().then(|| self.url.replace(QUERY, &percent_encode_query(query)))
    }
}

/// The actions a fresh install offers: three search engines and three AI tools.
pub fn defaults() -> Vec<WebTool> {
    vec![
        WebTool::new("Google", "https://www.google.com/search?q={query}"),
        WebTool::new("DuckDuckGo", "https://duckduckgo.com/?q={query}"),
        WebTool::new("Bing", "https://www.bing.com/search?q={query}"),
        WebTool::new("ChatGPT", "https://chatgpt.com/?q={query}"),
        WebTool::new("Gemini", "https://gemini.google.com/app?q={query}"),
        WebTool::new("Perplexity", "https://www.perplexity.ai/search?q={query}"),
    ]
}

/// Read the list from persisted settings. `None` when there is no usable list (missing, not an
/// array, or every entry invalid), so the caller keeps its defaults.
pub fn restore(value: &serde_json::Value) -> Option<Vec<WebTool>> {
    let entries = value.as_array()?;
    let tools: Vec<WebTool> = entries
        .iter()
        .filter_map(|entry| serde_json::from_value::<WebTool>(entry.clone()).ok())
        .map(|mut tool| {
            tool.label = tool.label.trim().to_owned();
            tool.url = tool.url.trim().to_owned();
            tool
        })
        .filter(WebTool::is_valid)
        .take(MAX_TOOLS)
        .collect();
    (!tools.is_empty()).then_some(tools)
}

/// Percent-encode a search query for a URL. Keeps the unreserved characters of RFC 3986 and turns
/// a space into `+`, which every search engine accepts. Self-contained so this needs no dependency.
pub fn percent_encode_query(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{MAX_TOOLS, WebTool, defaults, percent_encode_query, restore};

    #[test]
    fn query_encoding_keeps_unreserved_and_escapes_the_rest() {
        assert_eq!(percent_encode_query("quick"), "quick");
        assert_eq!(percent_encode_query("The quick brown fox"), "The+quick+brown+fox");
        assert_eq!(percent_encode_query("a&b=c?"), "a%26b%3Dc%3F");
        assert_eq!(percent_encode_query("é"), "%C3%A9", "non-ASCII is UTF-8, byte by byte");
        assert_eq!(percent_encode_query("x~y_z.1-2"), "x~y_z.1-2");
    }

    #[test]
    fn a_valid_template_fills_the_query_placeholder() {
        let tool = WebTool::new("Google", "https://www.google.com/search?q={query}");
        assert_eq!(tool.url_for("quick fox").as_deref(), Some("https://www.google.com/search?q=quick+fox"));
        assert_eq!(WebTool::new("No placeholder", "https://example.com/").url_for("x"), None);
        assert_eq!(WebTool::new("", "https://example.com/?q={query}").url_for("x"), None);
    }

    #[test]
    fn restore_keeps_only_valid_actions_and_falls_back_when_empty() {
        let tools = restore(&serde_json::json!([
            {"label": "  Google  ", "url": "https://www.google.com/search?q={query}"},
            {"label": "Broken", "url": "https://example.com/"},
            {"label": "", "url": "https://example.com/?q={query}"},
        ]));
        assert_eq!(tools, Some(vec![WebTool::new("Google", "https://www.google.com/search?q={query}")]));

        assert_eq!(restore(&serde_json::json!([])), None);
        assert_eq!(restore(&serde_json::json!([{"label": "Broken", "url": "https://example.com/"}])), None);
        assert_eq!(restore(&serde_json::Value::Null), None);
    }

    #[test]
    fn the_defaults_are_valid_and_capped() {
        let tools = defaults();
        assert!(tools.len() <= MAX_TOOLS);
        assert!(tools.iter().all(WebTool::is_valid));
        assert_eq!(tools[0].label, "Google", "the first action is the default");
    }
}
