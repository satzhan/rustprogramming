enum ContentType {
    Heading(String),
    Paragraph(String),
}

struct HtmlElement {
    content: ContentType,
}

impl HtmlElement {
    fn new(content_type: ContentType) -> Self {
        HtmlElement { content: content_type }
    }

    fn render(&self) -> String {
        match &self.content {
            ContentType::Heading(text) => format!("<h1>{}</h1>", text),
            ContentType::Paragraph(text) => format!("<p>{}</p>", text),
        }
    }
}

fn main() {
    let header = HtmlElement::new(ContentType::Heading(String::from("Welcome to My Website")));
    let paragraph = HtmlElement::new(ContentType::Paragraph(String::from("This is a simple HTML generator.")));

    println!("{}", header.render());
    println!("{}", paragraph.render());
}