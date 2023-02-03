use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use directories::ProjectDirs;
use futures_util::StreamExt;
use nostr_sdk::prelude::*;
use reqwest::{Client, Url};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone, Default)]
struct Status {
    downloading: HashSet<Url>,
}

pub enum DownloadResult {
    File(PathBuf),
    Dowloading,
}

impl DownloadResult {
    pub fn file(&self) -> Option<PathBuf> {
        match self {
            DownloadResult::File(f) => Some(f.clone()),
            DownloadResult::Dowloading => None,
        }
    }
}

#[derive(Clone)]
pub struct Download(Arc<DownloadInner>);

pub struct DownloadInner {
    dirs: ProjectDirs,
    http: Client,
    status: Arc<Mutex<Status>>,
}

impl Download {
    pub fn new(dirs: ProjectDirs) -> Download {
        Download(Arc::new(DownloadInner {
            dirs,
            http: Default::default(),
            status: Default::default(),
        }))
    }

    pub async fn cached(&self, url: &Url) -> Option<PathBuf> {
        let url_s = url.to_string();
        let filename = sha256::Hash::hash(url_s.as_bytes()).to_string();
        let file = self.0.dirs.cache_dir().join("bitmaps").join(filename);

        if file.is_file() {
            Some(file)
        } else {
            None
        }
    }

    pub async fn cached_file(&self, url: &Url) -> DownloadResult {
        let url_s = url.to_string();
        let filename = sha256::Hash::hash(url_s.as_bytes()).to_string();

        let cache = self.0.dirs.cache_dir().join("bitmaps");
        tokio::fs::create_dir_all(&cache).await.unwrap();
        let file = cache.join(&filename);

        let downloading = self.0.status.lock().await.downloading.contains(url);

        if downloading {
            info!("File from {} is already in cache", url_s);
            DownloadResult::Dowloading
        } else if file.is_file() {
            info!(
                "File from {} is already in cache as {:?}",
                url_s,
                file.file_name()
            );
            DownloadResult::File(file)
        } else {
            self.0.status.lock().await.downloading.insert(url.clone());

            let tmp = cache.join(format!("{filename}.part"));
            info!("Downloading {} to {:?}", url_s, tmp);

            let mut f = tokio::fs::File::create(&tmp).await.unwrap();
            let response = self.0.http.get(url.clone()).send().await.unwrap();
            // let content_length = response.headers().get("content-length");
            let mut bytes = response.bytes_stream();

            while let Some(chunk) = bytes.next().await {
                let c = chunk.unwrap();
                // println!("{}", c.len());
                f.write_all(&c).await.unwrap();
            }

            tokio::fs::rename(&tmp, &file).await.unwrap();

            info!("Download of {} finished, cached as {:?}", url_s, file);

            self.0.status.lock().await.downloading.remove(url);

            DownloadResult::File(file)
        }
    }
}
