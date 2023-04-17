use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::Window;
use winit::window::WindowId;
use winit::window::WindowBuilder;
use std::rc::Rc;
use std::cell::RefCell;

use crate::error::BSResult;
use crate::ui::Image;
use crate::ui::ListItem;
use crate::ui::UserInterface;

use super::ev_loop::UserEvent;

pub struct BrowserSelectorUI<ItemStateType: Clone> {
  state: UIState<ItemStateType>,
}

#[derive(Default)]
pub struct Theme {
    white: String,
    #[allow(dead_code)]
    black: String,
    light_gray: String,
    dark_gray: String,
    accent: String,
}

pub struct UIState<T: Clone> {
    pub list: Vec<crate::ui::ListItem<T>>,
    pub predictions: Vec<crate::ui::ListItem<T>>,
    pub theme: Theme,
    pub window: Option<Window>,
    pub browser_selected_handler: Option<Rc<RefCell<Box<dyn FnMut(&str) -> ()>>>>,
}

impl<ItemStateType: Clone> UserInterface<ItemStateType> for BrowserSelectorUI<ItemStateType> {
  fn new() -> BSResult<BrowserSelectorUI<ItemStateType>> {
    let state = UIState {
      list: Vec::<ListItem<ItemStateType>>::new(),
      predictions: Vec::<ListItem<ItemStateType>>::new(),
      theme: Default::default(),
      window: Default::default(),
      browser_selected_handler: None,
    };

    Ok(BrowserSelectorUI { state })
  }

  fn create(&mut self, title: &str, event_loop: &EventLoop<UserEvent>) -> BSResult<()> { 
    let window = WindowBuilder::new()
      .with_title(title)
      .with_decorations(true)
      .with_always_on_top(true)
      .with_inner_size(winit::dpi::LogicalSize {
          height: 400 as i16,
          width: 400 as i16,
      })
      .with_resizable(false)
      .with_visible(false)
      .build(&event_loop)
      .expect("Failed to create the main window");
  
    self.state.window = Some(window);
    Ok(()) 
  }
  
  fn get_window_id(&self) -> WindowId {
      self.state.window.as_ref().expect("Mising main window.").id()
  }

  fn set_main_window_visible(&self, visible: bool) {
      self.state.window.as_ref().expect("No main window. ui::create needs to be called.").set_visible(visible)
  }

  fn center_window_on_cursor_monitor(&self) {
    
  }

  fn set_list(&mut self, list: &[ListItem<ItemStateType>]) -> BSResult<()> {
    Ok(())
  }
  fn set_url(&self, url: &str) -> BSResult<()> {
    Ok(())
  }

  fn update_layout_size(&self, size: &PhysicalSize<u32>) -> BSResult<()> {
    Ok(())
  }
  fn load_image(path: &str) -> BSResult<Image> {
    todo!()
  }

  fn select_list_item_by_index(&self, index: isize) -> BSResult<()> {
    Ok(())
  }
  fn get_selected_list_item_index(&self) -> BSResult<isize> {
    Ok(-1)
  }
  fn get_selected_list_item(&self) -> BSResult<Option<ListItem<ItemStateType>>> {
    todo!()
  }
  fn get_list_length(&self) -> BSResult<usize> {
    todo!()
  }
  fn prediction_set_is_loading(&self, is_loading: bool) -> BSResult<()> { Ok(()) }
  fn prediction_set_state(&mut self, list: &[ListItem<ItemStateType>], duration: &str) -> BSResult<()> { Ok(()) }
  fn prediction_get_state(&self) -> &[ListItem<ItemStateType>] { todo!() }

  fn on_browser_selected(
        &mut self,
        event_handler: impl FnMut(&str) -> () + 'static,
    ) -> BSResult<()> {
        self.state.browser_selected_handler = Some(Rc::new(RefCell::new(Box::new(event_handler))));
        let handler_ptr = self.state.browser_selected_handler.as_ref().unwrap().clone();
        
        // list_control.item_click(wrt::ItemClickEventHandler::new(
        //     move |_: &winrt::Object, event: &wrt::ItemClickEventArgs| -> winrt::Result<()> {
        //         let item_tag = ui_element_get_tag_as_string(&event.clicked_item()?)
        //             .unwrap()
        //             .unwrap();

                
        //         let mut ev_handler = handler_ptr.as_ref().borrow_mut();
        //         (ev_handler)(item_tag.as_str());

        //         Ok(())
        //     },
        // ))?;

        Ok(())
    }

  fn trigger_browser_selected(&self, uuid: &str) {
    if let Some(handler_ptr) = self.state.browser_selected_handler.as_ref() {
        handler_ptr.as_ref().borrow_mut()(uuid);
    }
  }

  fn destroy(&self) {}
}
