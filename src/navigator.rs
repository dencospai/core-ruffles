//! Embedded Loader.loadBytes operations need an executor even in offline mode.
use async_channel::{Receiver, Sender};
use indexmap::IndexMap;
use ruffle_core::{
    backend::navigator::{
        async_return, create_specific_fetch_error, ErrorResponse, NavigationMethod,
        NavigatorBackend, NullNavigatorBackend, OwnedFuture, Request, SuccessResponse,
    },
    loader::Error,
    socket::{SocketAction, SocketHandle},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use url::{ParseError, Url};

pub struct OfflineNavigator {
    inner: NullNavigatorBackend,
    root: PathBuf,
}

impl OfflineNavigator {
    pub fn new(inner: NullNavigatorBackend, root: &Path) -> Result<Self, String> {
        Ok(Self {
            inner,
            root: root.canonicalize().map_err(|e| e.to_string())?,
        })
    }

    fn local_path(&self, requested: &str) -> Result<PathBuf, String> {
        let mut url = self.resolve_url(requested).map_err(|e| e.to_string())?;
        if url.scheme() != "file" {
            return Err("Network resources are unavailable".into());
        }
        url.set_query(None);
        url.set_fragment(None);
        let path = url
            .to_file_path()
            .map_err(|_| "Invalid local resource URL")?
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&self.root) || !path.is_file() {
            return Err("Resource must be a file inside the game's directory".into());
        }
        Ok(path)
    }
}

impl NavigatorBackend for OfflineNavigator {
    fn navigate_to_url(
        &self,
        _url: &str,
        _target: &str,
        _vars_method: Option<(NavigationMethod, IndexMap<String, String>)>,
    ) {
    }

    fn fetch(&self, request: Request) -> OwnedFuture<Box<dyn SuccessResponse>, ErrorResponse> {
        match self.local_path(request.url()) {
            Ok(path) => {
                eprintln!(
                    "[ruffle_libretro] Loading local resource: {}",
                    path.display()
                );
                self.inner.fetch(request)
            }
            Err(error) => {
                eprintln!(
                    "[ruffle_libretro] Resource unavailable: {} ({error})",
                    request.url()
                );
                async_return(Err(create_specific_fetch_error(&error, request.url(), "")))
            }
        }
    }

    fn resolve_url(&self, url: &str) -> Result<Url, ParseError> {
        self.inner.resolve_url(url)
    }
    fn spawn_future(&mut self, future: OwnedFuture<(), Error>) {
        self.inner.spawn_future(future);
    }
    fn pre_process_url(&self, url: Url) -> Url {
        url
    }

    fn connect_socket(
        &mut self,
        host: String,
        port: u16,
        timeout: Duration,
        handle: SocketHandle,
        receiver: Receiver<Vec<u8>>,
        sender: Sender<SocketAction>,
    ) {
        self.inner
            .connect_socket(host, port, timeout, handle, receiver, sender);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruffle_core::backend::navigator::NullExecutor;
    #[test]
    fn assets_resolve_relative_to_game_without_directory_escape() {
        let root = std::env::temp_dir().join(format!("ruffle-assets-{}", std::process::id()));
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin/data.swf"), b"fixture").unwrap();
        let executor = NullExecutor::new();
        let inner = NullNavigatorBackend::with_base_path(&root, &executor).unwrap();
        let navigator = OfflineNavigator::new(inner, &root).unwrap();
        assert_eq!(
            navigator.local_path("bin/data.swf?x=1").unwrap(),
            root.join("bin/data.swf").canonicalize().unwrap()
        );
        assert!(navigator
            .local_path("https://example.com/data.swf")
            .is_err());
        assert!(navigator.local_path("../outside.swf").is_err());
        assert!(navigator.local_path("bin/missing.swf").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
