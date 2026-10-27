use ruffle_core::backend::storage::StorageBackend;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct FileStorage {
    directory: PathBuf,
}

impl FileStorage {
    pub fn new(save_directory: &Path) -> std::io::Result<Self> {
        let directory = save_directory.join("ruffle");
        std::fs::create_dir_all(&directory)?;
        Ok(Self { directory })
    }

    fn path(&self, name: &str) -> PathBuf {
        // Ruffle's key includes the movie origin/path. Hash it to retain that
        // isolation without interpreting untrusted names as filesystem paths.
        self.directory
            .join(format!("{:x}.sol", Sha256::digest(name.as_bytes())))
    }
}

impl StorageBackend for FileStorage {
    fn get(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(name)).ok()
    }

    fn put(&mut self, name: &str, value: &[u8]) -> bool {
        let path = self.path(name);
        let temporary = path.with_extension("tmp");
        match std::fs::write(&temporary, value).and_then(|()| std::fs::rename(&temporary, &path)) {
            Ok(()) => true,
            Err(error) => {
                eprintln!("[ruffle_libretro] Failed to save SharedObject: {error}");
                false
            }
        }
    }

    fn remove_key(&mut self, name: &str) {
        let _ = std::fs::remove_file(self.path(name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_objects_persist_and_cannot_escape_save_directory() {
        let root = std::env::temp_dir().join(format!("ruffle-storage-{}", std::process::id()));
        let mut storage = FileStorage::new(&root).unwrap();
        let key = "../../outside:game/score";
        assert_eq!(storage.path(key).parent().unwrap(), root.join("ruffle"));
        assert!(storage.put(key, b"first"));
        assert!(storage.put(key, b"updated"));
        drop(storage);
        let mut storage = FileStorage::new(&root).unwrap();
        assert_eq!(storage.get(key), Some(b"updated".to_vec()));
        assert_eq!(storage.get("other-game/score"), None);
        storage.remove_key(key);
        assert_eq!(storage.get(key), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
