use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest;
use select::document::Document;
use select::node::Node;
use select::predicate::{Attr, Class, Name, Predicate};
use tokio;

struct StreetQuery {
    street: String,
    value: String,
}

impl StreetQuery {
    fn from(node: Node) -> Option<StreetQuery> {
        if let Some(value) = node.attr("value") {
            return Some(StreetQuery {
                street: node.text().trim().to_string(),
                value: value.to_string(),
            });
        }
        None
    }
}

struct Client {
    client: reqwest::Client,
    url: &'static str,
    date_expr: Regex,
}

impl Client {
    fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            url: "https://web6.karlsruhe.de/service/abfall/akal/akal.php",
            date_expr: Regex::new(r"(\d\d\.\d\d\.2021)")?,
        })
    }

    async fn queries(&self) -> Result<Vec<StreetQuery>> {
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
            .filter_map(|node| StreetQuery::from(node))
            .collect::<Vec<_>>())
    }

    async fn get_date(&self, query: &StreetQuery) -> Result<String> {
        let data = [
            ("anzeigen", "anzeigen"),
            ("strasse", &query.value),
            ("hausnr", ""),
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
            .ok_or(anyhow!("<div id='foo'><tbody> not found"))?
            .last_child() // last <tr>
            .ok_or(anyhow!("<tr> not found"))?
            .find(Name("td"))
            .skip(2)
            .next()
            .ok_or(anyhow!("third <td> not found"))?
            .text();

        Ok(self
            .date_expr
            .captures(&node)
            .ok_or(anyhow!("No date found for {}", query.street))?
            .get(0)
            .ok_or(anyhow!("foo"))?
            .as_str()
            .to_owned())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new()?;

    for query in client.queries().await? {
        match client.get_date(&query).await {
            Ok(date) => {
                println!("{};{}", query.street, date);
            }
            Err(_) => {
                println!("{};n/a", query.street);
            }
        }
    }

    Ok(())
}
