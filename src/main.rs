use std::env;
use std::io::Read;
use std::num::NonZeroUsize;

use reqwest::{Client, Response};
use url::percent_encoding::{utf8_percent_encode, QUERY_ENCODE_SET};

struct Video {
    id: String,
    title: String,
}

fn main() {
    let page = NonZeroUsize::new(1).unwrap();

    // get youtube query from arguments
    let query = query_string()
        .expect("no query string passed as argument");

    // download youtube page
    let mut rsp = yt_get(page, &query)
        .expect("failed to download youtube page");

    // parse the youtube page
    let mut doc_str = String::new();
    rsp.read_to_string(&mut doc_str)
        .expect("failed to read youtube page into String");

    // ignore sig pipe
    unsafe {
        use nix::sys::signal;

        signal::signal(signal::SIGPIPE, signal::SigHandler::SigDfl)
            .expect("failed to ignore SIGPIPE");
    }

    // begin scraping :^)
    find_videos(&doc_str, |v| {
        println!("{} | https://www.youtube.com/watch?v={}", v.title, v.id);
    })
}

// stolen from https://github.com/joetats/youtube_search/blob/master/youtube_search/__init__.py
fn find_videos<F>(doc_str: &str, f: F)
where
    F: Fn(Video),
{
    static SEARCH: &str = r#"window["ytInitialData"]"#;

    let start = doc_str.find(SEARCH)
        .map(|index| index + SEARCH.len() + 3)
        .expect("failed to find initial data");
    let end = doc_str[start..].find("};")
        .map(|index| index + start + 1)
        .expect("failed to find end index?!");
    let videos = ajson::get(&doc_str[start..end], "contents.twoColumnSearchResultsRenderer.primaryContents.sectionListRenderer.contents.0.itemSectionRenderer.contents")
        .map(|value| value.to_vec())
        .expect("couldn't find videos");
    for video_value in videos {
        let video = match video_value {
            ajson::Value::Object(_) => {
                let id = video_value.get("videoRenderer.videoId")
                    .and_then(|id| match id {
                        ajson::Value::String(id) => Some(id),
                        _ => None,
                    });
                let title = video_value.get("videoRenderer.title.runs.0.text")
                    .and_then(|id| match id {
                        ajson::Value::String(id) => Some(id),
                        _ => None,
                    });
                match id.and_then(|id| title.and_then(|title| Some((id, title)))) {
                    Some((id, title)) => Video { id, title },
                    _ => continue,
                }
            },
            _ => panic!("expected object, but got something else :("),
        };
        f(video);
    }
}

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

fn yt_get(page: NonZeroUsize, query: &str) -> reqwest::Result<Response> {
    static YT_BASE: &str = "https://www.youtube.com/results";

    let q = format!("{}?search_query={}&page={}",
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
