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
}

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
            Some(mt) if mt == media_type!(TEXT / HTML) => html_preview(response).await,
            _ => Preview::unknown(response.url().clone()),
        }
    }
}

async fn html_preview(response: Response) -> Preview {
    let url = response.url().clone();
    let body = response.text().await.ok();
    let html = body.and_then(|html| HTML::from_string(html, Some(url.to_string())).ok());

    match html {
        None => Preview::unknown(url),
        Some(html) => Preview {
            kind: PreviewKind::Webpage,
            url,
            title: html.title,
            description: html.description,
            thumbnail: None,
            error: None,
        },
    }
}

async fn image_preview(response: Response) -> Preview {
    Preview {
        kind: PreviewKind::Image,
        url: response.url().clone(),
        title: None,
        description: None,
        thumbnail: None,
        error: None,
    }
}
