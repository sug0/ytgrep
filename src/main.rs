use std::env;
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::io::{self, Read, Write, BufWriter};

use reqwest::{Client, Response};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

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

    // begin scraping :^)
    let stdout = io::stdout();
    let mut stdout_lock = BufWriter::new(stdout.lock());

    find_videos(&doc_str, |v| {
        writeln!(&mut stdout_lock, "{} | https://youtu.be/{}", v.title, v.id).is_ok()
    });

    let _ = stdout_lock.flush();
}

// stolen from https://github.com/joetats/youtube_search/blob/master/youtube_search/__init__.py
fn find_videos<F>(doc_str: &str, mut f: F)
where
    F: FnMut(Video) -> bool,
{
    const SEARCH1: (&str, usize) = ("var ytInitialData =", 1);
    const SEARCH2: (&str, usize) = ("// scraper_data_begin", 21);
    const SEARCH3: (&str, usize) = (r#"window["ytInitialData"]"#, 3);

    let start = doc_str.find(SEARCH1.0)
        .map(|index| index + SEARCH1.0.len() + SEARCH1.1)
        .or_else(|| doc_str
            .find(SEARCH2.0)
            .map(|index| index + SEARCH2.0.len() + SEARCH2.1))
        .or_else(|| doc_str
            .find(SEARCH3.0)
            .map(|index| index + SEARCH3.0.len() + SEARCH3.1))
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
        if !f(video) {
            break;
        }
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

    static USER_AGENT: &str = "AdsBot-Google (+http://www.google.com/adsbot.html)";
    //static USER_AGENT: &str = "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";

    let q = utf8_percent_encode(query, NON_ALPHANUMERIC);
    let qstr: Cow<'_, str> = q.into();
    let q = format!("{}?search_query={}&page={}",
        YT_BASE,
        qstr,
        page.get());

    Client::builder()
        .gzip(true)
        .build()?
        .get(q.as_str())
        .header("User-Agent", USER_AGENT)
        .header("Host", "www.youtube.com")
        .header("Cookie", "")
        .header("Referer", q.as_str())
        .header("Accept", "*/*")
        .header("Accept-Encoding", "gzip")
        .header("Accept-Language", "en-US,en;q=0.5")
        .header("Cache-Control", "max-age=0")
        .header("DNT", "1")
        .header("Connection", "close")
        .send()
}
