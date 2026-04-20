#[derive(Debug)]
pub enum Content {
    Text(String),
    Image { url: String, alt: String },
    Link { label: String, href: String },
}

pub fn render(c: Content) -> String {
    match c {
        Content::Text(s) => format!("<p>{}</p>", s),
        Content::Image { url, alt } => format!("<img src=\"{}\" alt=\"{}\"/>", url, alt),
        Content::Link { label, href } => format!("<a href=\"{}\">{}</a>", href, label),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_text() {
        assert_eq!(render(Content::Text("hi".into())), "<p>hi</p>");
    }
}