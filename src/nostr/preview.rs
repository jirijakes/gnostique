use gtk::gdk;
use mediatype::names::IMAGE;
use mediatype::{media_type, MediaTypeBuf};
use nostr_sdk::Url;
use reqwest::Response;
use webpage::HTML;

#[derive(Debug, Clone, sqlx::Decode)]
pub enum PreviewKind {
    Image,
    Webpage,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Preview {
    kind: PreviewKind,
    url: Url,
    title: Option<String>,
    description: Option<String>,
    // TODO: Try not to use GTK-specific type here.
    thumbnail: Option<gdk::Texture>,
    error: Option<String>,
}

impl Preview {
    pub const fn new(
        url: Url,
        kind: PreviewKind,
        title: Option<String>,
        description: Option<String>,
        thumbnail: Option<gdk::Texture>,
        error: Option<String>,
    ) -> Self {
        Self {
            kind,
            url,
            title,
            description,
            thumbnail,
            error,
        }
    }

    pub const fn unknown(url: Url) -> Preview {
        Preview::new(url, PreviewKind::Unknown, None, None, None, None)
    }

    pub const fn error(url: Url, error: String) -> Preview {
        Preview::new(url, PreviewKind::Unknown, None, None, None, Some(error))
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    // TODO: Move to Download so we can reuse client and caching and stats.
    pub async fn create(url: Url) -> Preview {
        let orig_url = url.clone();
        match reqwest::get(url).await {
            Err(err) => Preview::error(orig_url, err.to_string()),
            Ok(response) => make_preview(response).await,
        }
    }

    pub fn thumbnail(&self) -> Option<&gdk::Texture> {
        self.thumbnail.as_ref()
    }

    pub fn description(&self) -> Option<&String> {
        self.description.as_ref()
    }
}

/// Generates preview of whatever a given HTTP response contains.
async fn make_preview(response: Response) -> Preview {
    let status = response.status();
    if status.is_server_error() || status.is_client_error() {
        Preview::error(response.url().clone(), response.status().to_string())
    } else {
        let mt = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .and_then(|ct| ct.parse::<MediaTypeBuf>().ok());

        match mt {
            Some(mt) if mt.ty() == IMAGE => image_preview(response).await,
            Some(mt) if mt.essence() == media_type!(TEXT / HTML) => html_preview(response).await,
            Some(mt) => {
                dbg!(mt);
                Preview::unknown(response.url().clone())
            }
            _ => Preview::unknown(response.url().clone()),
        }
    }
}

/// Generates preview of a webpage.
async fn html_preview(response: Response) -> Preview {
    let url = response.url().clone();
    let body = response.text().await.ok();
    let html = body.and_then(|html| HTML::from_string(html, Some(url.to_string())).ok());

    match html {
        None => Preview::unknown(url),
        Some(html) => {
            let og = &html.opengraph;
            let image_url = {
                og.images
                    .iter()
                    .min_by(|obj1, obj2| {
                        let w1 = obj1
                            .properties
                            .get("width")
                            .and_then(|w| w.parse::<u16>().ok());
                        let w2 = obj2
                            .properties
                            .get("width")
                            .and_then(|w| w.parse::<u16>().ok());

                        w1.cmp(&w2)
                    })
                    .and_then(|obj| Url::parse(&obj.url).ok())
            };

            let thumbnail = match image_url {
                Some(url) => {
                    let res = reqwest::get(url).await.ok();
                    match res {
                        Some(r) => r.bytes().await.ok().and_then(|b| {
                            gdk::Texture::from_bytes(&gtk::glib::Bytes::from(&b)).ok()
                        }),
                        None => None,
                    }
                }
                None => None,
            };

            Preview {
                kind: PreviewKind::Webpage,
                url,
                title: html.title,
                description: html.description,
                thumbnail,
                error: None,
            }
        }
    }
}

/// Generates preview of an image.
async fn image_preview(response: Response) -> Preview {
    let thumbnail = {
        let res = reqwest::get(response.url().clone()).await.ok();
        match res {
            Some(r) => r
                .bytes()
                .await
                .ok()
                .and_then(|b| gdk::Texture::from_bytes(&gtk::glib::Bytes::from(&b)).ok()),
            None => None,
        }
    };

    Preview {
        kind: PreviewKind::Image,
        url: response.url().clone(),
        title: None,
        description: None,
        thumbnail,
        error: None,
    }
}
