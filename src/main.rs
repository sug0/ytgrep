use std::env;
use std::io::BufReader;
use std::num::NonZeroUsize;

use reqwest::{Client, Response};
use url::percent_encoding::{utf8_percent_encode, QUERY_ENCODE_SET};
use select::{predicate::{self, Predicate}, document, node};

fn main() {
    let page = NonZeroUsize::new(1).unwrap();

    // get youtube query from arguments
    let query = query_string()
        .expect("no query string passed as argument");

    // download youtube page
    let mut rsp = yt_get(page, &query)
        .expect("failed to download youtube page");

    // parse the youtube page
    let doc = document::Document::from_read(&mut BufReader::new(&mut rsp))
        .expect("failed to parse youtube page");

    // begin scraping :^)
    find_video_nodes(&doc, |n| {
        let path = n.attr("href").unwrap();
        let title = n.attr("title").unwrap();
        println!("{} | https://www.youtube.com{}", title, path)
    })
}

#[inline]
fn query_string() -> Option<String> {
    // fetch arguments
    let args: Vec<_> = env::args()
        .skip(1)
        .collect();

    if args.is_empty() {
        return None
    }

    // allocate new string buffer
    let mut s = String::new();
    let n = args.len() - 1;

    // join arguments with a space
    for i in 0..n {
        s.push_str(&args[i]);
        s.push_str(" ")
    }
    s.push_str(&args[n]);

    // return the final query string
    Some(s)
}

#[inline]
fn find_video_nodes<'a, F>(doc: &'a document::Document, process: F)
where
    F: Fn(node::Node<'a>),
{
    let pred = predicate::Name("h3").child(
        predicate::Name("a")
            .and(predicate::Attr("title", ()))
            .and(predicate::Attr("dir", "ltr")),
    );
    doc.find(pred).for_each(process)
}

fn yt_get(page: NonZeroUsize, query: &str) -> reqwest::Result<Response> {
    static YT_BASE: &str = "https://www.youtube.com/results";

    let q = format!("{}?search_query={}&page={}&disable_polymer=1",
        YT_BASE,
        utf8_percent_encode(query, QUERY_ENCODE_SET).to_string(),
        page.get());

    Client::builder()
        .gzip(true)
        .build()?
        .get(q.as_str())
        .header("User-Agent", "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)")
        .header("Host", "www.youtube.com")
        .header("Referer", q.as_str())
        .header("Accept", "*/*")
        .header("Accept-Encoding", "gzip")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("Cache-Control", "max-age=0")
        .header("DNT", "1")
        .header("Connection", "close")
        .send()
}
