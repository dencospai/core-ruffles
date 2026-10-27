//! Keep the software pointer in sync with Flash Mouse.hide()/show().
//! All other UI services retain the existing NullUiBackend behavior.
use ruffle_core::{
    backend::ui::{
        DialogResultFuture, FileFilter, FontDefinition, FullscreenError, LanguageIdentifier,
        MouseCursor, MultiDialogResultFuture, NullUiBackend, UiBackend,
    },
    font::FontQuery,
};
use url::Url;

pub struct RetroUi {
    inner: NullUiBackend,
    visible: bool,
}

impl Default for RetroUi {
    fn default() -> Self {
        Self {
            inner: NullUiBackend::new(),
            visible: true,
        }
    }
}

impl UiBackend for RetroUi {
    fn mouse_visible(&self) -> bool {
        self.visible
    }
    fn set_mouse_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    // Player::mouse_cursor() is the authoritative shape used by the renderer.
    fn set_mouse_cursor(&mut self, _cursor: MouseCursor) {}
    fn clipboard_content(&mut self) -> String {
        self.inner.clipboard_content()
    }
    fn set_clipboard_content(&mut self, content: String) {
        self.inner.set_clipboard_content(content);
    }
    fn set_fullscreen(&mut self, full: bool) -> Result<(), FullscreenError> {
        self.inner.set_fullscreen(full)
    }
    fn display_root_movie_download_failed_message(&self, invalid: bool, error: String) {
        self.inner
            .display_root_movie_download_failed_message(invalid, error);
    }
    fn message(&self, message: &str) {
        self.inner.message(message);
    }
    fn open_virtual_keyboard(&self) {
        self.inner.open_virtual_keyboard();
    }
    fn close_virtual_keyboard(&self) {
        self.inner.close_virtual_keyboard();
    }
    fn language(&self) -> LanguageIdentifier {
        self.inner.language()
    }
    fn display_unsupported_video(&self, url: Url) {
        self.inner.display_unsupported_video(url);
    }
    fn load_device_font(&self, query: &FontQuery, register: &mut dyn FnMut(FontDefinition)) {
        self.inner.load_device_font(query, register);
    }
    fn sort_device_fonts(
        &self,
        query: &FontQuery,
        register: &mut dyn FnMut(FontDefinition),
    ) -> Vec<FontQuery> {
        self.inner.sort_device_fonts(query, register)
    }
    fn display_file_open_dialog(&mut self, filters: Vec<FileFilter>) -> Option<DialogResultFuture> {
        self.inner.display_file_open_dialog(filters)
    }
    fn display_file_open_dialog_multiple(
        &mut self,
        filters: Vec<FileFilter>,
    ) -> Option<MultiDialogResultFuture> {
        self.inner.display_file_open_dialog_multiple(filters)
    }
    fn display_file_save_dialog(
        &mut self,
        file_name: String,
        title: String,
    ) -> Option<DialogResultFuture> {
        self.inner.display_file_save_dialog(file_name, title)
    }
    fn close_file_dialog(&mut self) {
        self.inner.close_file_dialog();
    }
}
