use anyhow::{anyhow, Result};
use regex::Regex;
use select::document::Document;
use select::node::Node;
use select::predicate::{Attr, Class, Name, Predicate};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Pickup {
    pub street: String,
    pub date: String,
}

#[derive(Debug)]
pub struct Query {
    pub street: String,
    pub value: String,
}

impl Query {
    fn from(node: Node) -> Option<Query> {
        if let Some(value) = node.attr("value") {
            return Some(Query {
                street: node.text().trim().to_string(),
                value: value.to_string(),
            });
        }
        None
    }
}

pub struct Client {
    client: reqwest::Client,
    url: &'static str,
    date_expr: Regex,
}

impl Client {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            url: "https://web6.karlsruhe.de/service/abfall/akal/akal.php",
            date_expr: Regex::new(r"(\d\d\.\d\d\.2022)")?,
        })
    }

    pub async fn queries(&self) -> Result<Vec<Query>> {
        let text = self
            .client
            .post(self.url)
            .query(&[("von", "A"), ("bis", "[")])
            .send()
            .await?
            .text()
            .await?;

        let document = Document::from(text.as_str());

        Ok(document
            .find(Class("tab_body").descendant(Name("option")))
            .into_iter()
            .filter_map(Query::from)
            .collect::<Vec<_>>())
    }

    pub async fn get_pickup(&self, query: Query) -> Result<Pickup> {
        let data = [
            ("anzeigen", "anzeigen"),
            ("strasse", &query.value),
            ("hausnr", "1"),
        ];

        let text = self
            .client
            .post(self.url)
            .query(&[("von", "A"), ("bis", "[")])
            .form(&data)
            .send()
            .await?
            .text()
            .await?;

        let node = Document::from(text.as_str())
            .find(Attr("id", "foo").descendant(Name("tbody")))
            .next()
            .ok_or_else(|| anyhow!("<div id='foo'><tbody> not found"))?
            .last_child() // last <tr>
            .ok_or_else(|| anyhow!("<tr> not found"))?
            .find(Name("td"))
            .nth(2)
            .ok_or_else(|| anyhow!("third <td> not found"))?
            .text();

        Ok(Pickup {
            date: self
                .date_expr
                .captures(&node)
                .ok_or_else(|| anyhow!("No date found for {}", query.street))?
                .get(0)
                .ok_or_else(|| anyhow!("foo"))?
                .as_str()
                .to_owned(),
            street: query.street,
        })
    }
}
