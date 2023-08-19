use gtk::gdk;
use nostr_sdk::Url;
use reqwest::Response;

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
    thumbnail: Option<gdk::Texture>,
    error: Option<String>,
}

impl Preview {
    pub fn new(
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

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    // TODO: Move to Download
    pub async fn create(url: Url) -> Preview {
        let orig_url = url.clone();
        match reqwest::get(url).await {
            Err(err) => Preview {
                kind: PreviewKind::Unknown,
                url: orig_url,
                error: Some(err.to_string()),
                title: None,
                description: None,
                thumbnail: None,
            },
            Ok(response) => make_preview(response).await,
        }
    }
}

async fn make_preview(response: Response) -> Preview {
    let status = response.status();
    if status.is_server_error() || status.is_client_error() {
        Preview {
            kind: PreviewKind::Unknown,
            url: response.url().clone(),
            error: Some(response.status().to_string()),
            title: None,
            description: None,
            thumbnail: None,
        }
    } else {
        let content_type = response.headers().get("content-type");
        dbg!(content_type);
        Preview {
            kind: PreviewKind::Unknown,
            url: response.url().clone(),
            error: None,
            title: None,
            description: None,
            thumbnail: None,
        }
    }
}
