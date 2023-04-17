#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "macos")]
mod macos;

pub mod ev_loop;

use crate::error::BSResult;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{WindowId};

use std::rc::Rc;

/*
  This is a stub for seamlesly integrating multiple platforms (OSes)
*/
#[cfg(target_os = "windows")]
pub type Image = bindings::windows::ui::xaml::controls::Image;
#[cfg(target_os = "windows")]
pub use win::BrowserSelectorUI;

#[cfg(target_os = "macos")]
pub use macos::BrowserSelectorUI;
#[cfg(target_os = "macos")]
pub type Image = cacao::image::Image;

use self::ev_loop::UserEvent;
#[cfg(target_os = "windows")]
mod windows_desktop_window_xaml_source;

pub trait UserInterface<T: Clone> {
    fn new() -> BSResult<BrowserSelectorUI<T>>;
    fn create(&mut self, window_title: &str, event_loop: &EventLoop<UserEvent>) -> BSResult<()>;
    fn set_main_window_visible(&self, visible: bool);
    fn get_window_id(&self) -> WindowId;
    fn center_window_on_cursor_monitor(&self);

    fn set_list(&mut self, list: &[ListItem<T>]) -> BSResult<()>;
    fn set_url(&self, url: &str) -> BSResult<()>;

    fn update_layout_size(&self, size: &PhysicalSize<u32>) -> BSResult<()>;
    fn load_image(path: &str) -> BSResult<Image>;

    fn select_list_item_by_index(&self, index: isize) -> BSResult<()>;
    fn get_selected_list_item_index(&self) -> BSResult<isize>;
    fn get_selected_list_item(&self) -> BSResult<Option<ListItem<T>>>;
    fn get_list_length(&self) -> BSResult<usize>;
    fn prediction_set_is_loading(&self, is_loading: bool) -> BSResult<()>;
    fn prediction_set_state(&mut self, list: &[ListItem<T>], duration: &str) -> BSResult<()>;
    fn prediction_get_state(&self) -> &[ListItem<T>];

    fn on_browser_selected(
        &mut self,
        event_handler: impl FnMut(&str) -> () + 'static,
    ) -> BSResult<()>;
    fn trigger_browser_selected(&self, uuid: &str);

    fn destroy(&self);
}

#[derive(Clone)]
pub struct ListItem<T: Clone> {
    pub title: String,
    pub subtitle: String,
    pub image: Image,
    pub uuid: String,
    pub state: Rc<T>,
}
