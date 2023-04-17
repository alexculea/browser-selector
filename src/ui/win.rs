use std::cell::RefCell;
use std::convert::TryInto;
use std::mem::MaybeUninit;
use std::rc::Rc;

// For clarity purposes keep all WinRT imports under wrt::
// winrt is a different crate dealing with types for calling the imported resources
// TODO: Find a better name rather than `wrt` to avoid confusion btw `wrt` and `winrt`
mod wrt {
    pub use bindings::windows::ui::xaml::hosting::{
        DesktopWindowXamlSource, IDesktopWindowXamlSourceFactory, WindowsXamlManager,
    };

    pub use bindings::windows::storage::streams::{DataWriter, IBuffer, IDataWriterFactory};

    pub use bindings::windows::foundation::{
        IPropertyValue, IReference, IStringable, PropertyType, PropertyValue,
    };
    pub use bindings::windows::graphics::imaging::{
        BitmapAlphaMode, BitmapPixelFormat, ISoftwareBitmapFactory, SoftwareBitmap,
    };
    pub use bindings::windows::ui::view_management::{UIColorType, UISettings};
    pub use bindings::windows::ui::xaml::controls::{
        Button, ColumnDefinition, Grid, IButtonFactory, IGridFactory, IListBoxFactory,
        IListViewFactory, IRelativePanelFactory, IScrollViewerStatics, IStackPanelFactory, Image,
        ItemClickEventArgs, ItemClickEventHandler, ItemsControl, ListBox, ListView,
        ListViewSelectionMode, Orientation, Panel, ProgressRing, RelativePanel, RowDefinition,
        ScrollMode, ScrollViewer, StackPanel, TextBlock,
    };
    pub use bindings::windows::ui::xaml::interop::{TypeKind, TypeName};
    pub use bindings::windows::ui::xaml::media::imaging::{BitmapImage, SoftwareBitmapSource};
    pub use bindings::windows::ui::xaml::media::{ImageSource, SolidColorBrush};
    pub use bindings::windows::ui::xaml::{
        FrameworkElement, GridLength, GridUnitType, RoutedEventHandler, Thickness, UIElement,
        VerticalAlignment, Visibility,
    };
    pub use bindings::windows::ui::Color;
}

mod winapi {
    pub use winapi::ctypes::*;
    pub use winapi::shared::windef::{HGDIOBJ, HICON, HWND, POINT};
    pub use winapi::um::wingdi::{DeleteObject, GetBitmapBits, GetObjectW, BITMAP, DIBSECTION};
    pub use winapi::um::winuser::{
        GetCursorPos, GetIconInfo, SetWindowPos, UpdateWindow, ICONINFO, MONITORINFO,
    };
}

use crate::error::*;
use crate::os::{as_u8_slice, get_hwnd};
use crate::ui::windows_desktop_window_xaml_source::IDesktopWindowXamlSourceNative;

use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::platform::windows::IconExtWindows;
use winit::window::{Window, WindowBuilder, WindowId};

use winrt::ComInterface;

use crate::ui::Image;
use crate::ui::ListItem;
use crate::ui::UserInterface;

use super::ev_loop::UserEvent;
pub struct BrowserSelectorUI<ItemStateType: Clone> {
    state: UIState<ItemStateType>,
}

pub struct XamlIslandWindow {
    pub hwnd_parent: *mut std::ffi::c_void,

    // the container that draws the DirectComposition stuff to render
    // the modern Windows UI
    pub hwnd: *mut std::ffi::c_void,

    // COM class having the DirectComposition resources
    // has to be initialized first and destroyed last
    pub win_xaml_mgr: wrt::WindowsXamlManager,

    // DesktopWindowXamlSource COM base class
    pub desktop_source: wrt::DesktopWindowXamlSource,

    // IDesktopWindowXamlSource COM derived from DesktopWindowXamlSource above
    // and contains the 'attach' function for using it with existing HWND
    pub idesktop_source: IDesktopWindowXamlSourceNative,
}

impl Default for XamlIslandWindow {
    fn default() -> XamlIslandWindow {
        unsafe {
            XamlIslandWindow {
                hwnd_parent: std::ptr::null_mut(),
                hwnd: std::ptr::null_mut(),
                idesktop_source: std::mem::zeroed(),
                desktop_source: std::mem::zeroed(),
                win_xaml_mgr: std::mem::zeroed(),
            }
        }
    }
}

#[derive(Default)]
pub struct Theme {
    white: wrt::Color,
    #[allow(dead_code)]
    black: wrt::Color,
    light_gray: wrt::Color,
    dark_gray: wrt::Color,
    accent: wrt::Color,
}

pub struct UIState<T: Clone> {
    pub xaml_isle: XamlIslandWindow,
    pub list: Vec<crate::ui::ListItem<T>>,
    pub predictions: Vec<crate::ui::ListItem<T>>,
    pub container: wrt::Panel,
    pub theme: Theme,
    pub window: Option<Window>,
    pub browser_selected_handler: Option<Rc<RefCell<Box<dyn FnMut(&str) -> ()>>>>,
}

const LIST_CONTROL_NAME: &str = "browserList";
const URL_CONTROL_NAME: &str = "urlControl";
const HEADER_PANEL_NAME: &str = "headerPanel";

impl<ItemStateType: Clone> UserInterface<ItemStateType> for BrowserSelectorUI<ItemStateType> {
    fn new() -> BSResult<Self> {
        // TODO: Correct error handling
        // unsafe { initialize_runtime_com()?; }

        // Initialize WinUI XAML before creating the winit EventLoop
        // or winit throws: thread 'main'
        // panicked at 'either event handler is re-entrant (likely), or no event
        // handler is registered (very unlikely)'
        let state = UIState {
            xaml_isle: init_win_ui_xaml()?,
            list: Vec::<ListItem<ItemStateType>>::new(),
            predictions: Vec::<ListItem<ItemStateType>>::new(),
            container: wrt::Panel::default(),
            theme: create_theme()?,
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

        let size = window.inner_size();
        self.state.xaml_isle.hwnd = attach_window_to_xaml(&window, &mut self.state.xaml_isle)?;
        update_xaml_island_size(&self.state.xaml_isle, size)?;
        unsafe {
            winapi::UpdateWindow(self.state.xaml_isle.hwnd_parent as winapi::HWND);
        }

        let ui_container = create_ui(&self.state, &self.state.theme)?;

        self.state
            .xaml_isle
            .desktop_source
            .set_content(ui_container.to_owned())?;
        self.state.container = ComInterface::query::<wrt::Panel>(&ui_container);

        window.set_window_icon(Some(
            winit::window::Icon::from_resource(
                1,
                Some(winit::dpi::PhysicalSize {
                    width: 16 as u32,
                    height: 16 as u32,
                }),
            )
            .unwrap(),
        ));

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
        center_window_on_cursor_monitor(&self.state.window)
            .expect("Couldn't center main window on the mouse monitor.");
    }

    fn destroy(&self) {
        #![allow(unused_must_use)]
        self.state.xaml_isle.desktop_source.close();
    }

    fn update_layout_size(&self, size: &PhysicalSize<u32>) -> BSResult<()> {
        update_xaml_island_size(&self.state.xaml_isle, *size)?;

        Ok(())
    }

    fn set_list(&mut self, list: &[ListItem<ItemStateType>]) -> BSResult<()> {
        if let Some(ui_element) =
            recursive_find_child_by_tag(&self.state.container, LIST_CONTROL_NAME)?
        {
            let list_view: wrt::ListView = ComInterface::query(&ui_element);
            self.state.list = list.clone().to_vec();
            set_listview_items(&list_view, list, &self.state.theme)?;
        }

        Ok(())
    }

    fn set_url(&self, new_url: &str) -> BSResult<()> {
        if let Some(ui_element) =
            recursive_find_child_by_tag(&self.state.container, URL_CONTROL_NAME)?
        {
            let text_block = ComInterface::query::<wrt::TextBlock>(&ui_element);
            text_block.set_text(new_url)?;
        }

        Ok(())
    }

    fn load_image(path: &str) -> BSResult<Image> {
        let hicon = crate::os::get_exe_file_icon(path)?;
        let bmp = hicon_to_software_bitmap(hicon)?;

        match software_bitmap_to_xaml_image(bmp) {
            Ok(image) => Ok(image),
            Err(winrt_error) => Err(BSError::from(winrt_error)),
        }
    }

    fn select_list_item_by_index(&self, index: isize) -> BSResult<()> {
        let list_control: wrt::ListView =
            recursive_find_child_by_tag(&self.state.container, LIST_CONTROL_NAME)
                .unwrap()
                .unwrap()
                .query();

        list_control.set_selected_index(index as i32)?;

        Ok(())
    }

    fn get_selected_list_item_index(&self) -> BSResult<isize> {
        let list_control: wrt::ListView =
            recursive_find_child_by_tag(&self.state.container, LIST_CONTROL_NAME)
                .unwrap()
                .unwrap()
                .query();

        Ok(list_control.selected_index()? as isize)
    }
    fn get_selected_list_item(&self) -> BSResult<Option<ListItem<ItemStateType>>> {
        let selected_index = self.get_selected_list_item_index()?;
        if selected_index < 0 {
            return Ok(None);
        }

        let cloned_item = self.state.list[selected_index as usize].clone();
        Ok(Some(cloned_item))
    }

    fn on_browser_selected(
        &mut self,
        event_handler: impl FnMut(&str) -> () + 'static,
    ) -> BSResult<()> {
        self.state.browser_selected_handler = Some(Rc::new(RefCell::new(Box::new(event_handler))));
        let handler_ptr = self.state.browser_selected_handler.as_ref().unwrap().clone();
        let list_control: wrt::ListView =
            recursive_find_child_by_tag(&self.state.container, LIST_CONTROL_NAME)
                .unwrap()
                .unwrap()
                .query();
        list_control.set_is_item_click_enabled(true)?;
        list_control.item_click(wrt::ItemClickEventHandler::new(
            move |_: &winrt::Object, event: &wrt::ItemClickEventArgs| -> winrt::Result<()> {
                let item_tag = ui_element_get_tag_as_string(&event.clicked_item()?)
                    .unwrap()
                    .unwrap();

                
                let mut ev_handler = handler_ptr.as_ref().borrow_mut();
                (ev_handler)(item_tag.as_str());

                Ok(())
            },
        ))?;

        Ok(())
    }

    fn trigger_browser_selected(&self, uuid: &str) {
        if let Some(handler_ptr) = self.state.browser_selected_handler.as_ref() {
            handler_ptr.as_ref().borrow_mut()(uuid);
        }
    }

    fn get_list_length(&self) -> BSResult<usize> {
        Ok(self.state.list.len())
    }

    fn prediction_set_is_loading(&self, is_loading: bool) -> BSResult<()> {
        let get_spinner_visibility = || { if is_loading { wrt::Visibility::Visible } else { wrt::Visibility::Collapsed } };
        let predictions_panel_opt =
            recursive_find_child_by_tag(&self.state.container, "predictions_panel")?;
        if predictions_panel_opt.is_none() {
            let predictions_panel =
                winrt::factory::<wrt::StackPanel, wrt::IStackPanelFactory>()?
                    .create_instance(winrt::Object::default(), &mut winrt::Object::default())?;
            ui_element_set_string_tag(&predictions_panel, "predictions_panel")?;
            predictions_panel.set_min_height(70.0)?;
            predictions_panel.set_padding(wrt::Thickness { top: 15.0, right: 15.0, bottom: 15.0, left: 15.0 })?;

            let title = wrt::TextBlock::new()?;
            title.set_text("Predictions (...)")?;
            ui_element_set_string_tag(&title, "predictions_title")?;

            let spinner = wrt::ProgressRing::new()?;
            spinner.set_is_active(is_loading)?;
            ui_element_set_string_tag(&spinner, "predictions_spinner")?;
            spinner.set_visibility(get_spinner_visibility())?;

            predictions_panel.children()?.append(title)?;
            predictions_panel.children()?.append(spinner)?;

            wrt::Grid::set_row(&predictions_panel, 2)?;
            wrt::Grid::set_column(&predictions_panel, 0)?;
            self.state.container.children()?.append(predictions_panel)?;
        } else {
            let Some(predictions_panel) = predictions_panel_opt else { bail!("Panel is not None but not Some either?"); };
            let spinner_elm =
                recursive_find_child_by_tag(&predictions_panel, "predictions_spinner")?.unwrap();
            let spinner = spinner_elm.query::<wrt::ProgressRing>();
            spinner.set_is_active(is_loading)?;
            spinner.set_visibility(get_spinner_visibility())?;
        }

        Ok(())
    }

    fn prediction_set_state(
        &mut self,
        list: &[ListItem<ItemStateType>],
        duration: &str,
    ) -> BSResult<()> {
        self.prediction_set_is_loading(false)?;
        
        let mut predictions_list_panel =
            recursive_find_child_by_tag(&self.state.container, "predictions_list_panel")?;
        if predictions_list_panel.is_none() {
            let predictions_panel =
                recursive_find_child_by_tag(&self.state.container, "predictions_panel")?.unwrap();
            let predictions_panel = predictions_panel.query::<wrt::StackPanel>();
            let list_container = winrt::factory::<wrt::StackPanel, wrt::IStackPanelFactory>()?
                .create_instance(winrt::Object::default(), &mut winrt::Object::default())?;
            ui_element_set_string_tag(&list_container, "prediction_list_panel")?;
            predictions_panel.children()?.append(list_container.clone())?;
            predictions_list_panel = Some(list_container.query::<wrt::UIElement>());
        }
        let predictions_list_panel = predictions_list_panel.unwrap();
        let predictions_list_panel = predictions_list_panel.query::<wrt::StackPanel>();
        predictions_list_panel.children()?.clear()?;

        let predictions_title =
            recursive_find_child_by_tag(&self.state.container, "predictions_title")?.unwrap();
        let predictions_title = predictions_title.query::<wrt::TextBlock>();
        let predictions_title_str = format!("Predictions ({})", duration);
        predictions_title.set_text(predictions_title_str.as_str())?;

        let key_shortcuts = ["space", "backspace"];
        let mut index = 0;
        list.iter().try_for_each::<_, BSResult<()>>(| list_item | {
            let text_block = wrt::TextBlock::new()?;
            let mut text = list_item.title.clone();
            if let Some(key) = key_shortcuts.iter().take(index + 1).last() {
                text = format!("{} ({})", text, key);
            }
            text_block.set_text(text.as_str())?;
            predictions_list_panel.children()?.append(text_block)?;

            index += 1;
            Ok(())
        })?;

        self.state.predictions = Vec::from(list);
        
        Ok(())
    }

    fn prediction_get_state(&self) -> &[ListItem<ItemStateType>] {
        return &self.state.predictions;   
    }
}

pub fn init_win_ui_xaml() -> winrt::Result<XamlIslandWindow> {
    use winrt::Object;
    let mut xaml_isle = XamlIslandWindow::default();
    xaml_isle.win_xaml_mgr = wrt::WindowsXamlManager::initialize_for_current_thread()?;
    xaml_isle.desktop_source =
        winrt::factory::<wrt::DesktopWindowXamlSource, wrt::IDesktopWindowXamlSourceFactory>()?
            .create_instance(Object::default(), &mut Object::default())?;
    xaml_isle.idesktop_source = xaml_isle.desktop_source.clone().into();

    Ok(xaml_isle)
}

pub fn attach_window_to_xaml(
    window: &winit::window::Window,
    xaml_isle: &mut XamlIslandWindow,
) -> winrt::Result<*mut std::ffi::c_void> {
    xaml_isle.hwnd_parent = get_hwnd(window) as *mut std::ffi::c_void;

    xaml_isle
        .idesktop_source
        .attach_to_window(xaml_isle.hwnd_parent)?;
    return xaml_isle.idesktop_source.get_window_handle();
}

pub fn update_xaml_island_size(
    xaml_isle: &XamlIslandWindow,
    size: winit::dpi::PhysicalSize<u32>,
) -> winrt::Result<()> {
    unsafe {
        winapi::SetWindowPos(
            xaml_isle.hwnd as winapi::HWND,
            std::ptr::null_mut(),
            0,
            0,
            size.width as i32,
            size.height as i32,
            0x40,
        );
    }

    Ok(())
}

pub fn create_ui<T: Clone>(ui: &UIState<T>, theme: &Theme) -> winrt::Result<wrt::UIElement> {
    let header_panel = create_header("You are about to open:", "", &theme)?;
    let list = create_list(&ui.list, &theme)?;
    let grid = create_main_layout_grid(&theme)?;

    wrt::Grid::set_row(
        ComInterface::query::<wrt::FrameworkElement>(&header_panel),
        0,
    )?;
    wrt::Grid::set_column(
        ComInterface::query::<wrt::FrameworkElement>(&header_panel),
        0,
    )?;
    wrt::Grid::set_row(&ComInterface::query::<wrt::FrameworkElement>(&list), 1)?;
    wrt::Grid::set_column(ComInterface::query::<wrt::FrameworkElement>(&list), 0)?;

    grid.children()?.append(header_panel)?;
    grid.children()?.append(list)?;

    Ok(grid.into())
}

/// Creates a WinUI Grid control with a single column and two rows
/// fit to be used for presentation in the main window where the top
/// row has the action intro text (ie. "You are about to open x URL")
/// and the bottom row has the list of browsers available.
pub fn create_main_layout_grid(theme: &Theme) -> winrt::Result<wrt::Grid> {
    let grid = winrt::factory::<wrt::Grid, wrt::IGridFactory>()?
        .create_instance(winrt::Object::default(), &mut winrt::Object::default())?;
    let column_definition = wrt::ColumnDefinition::new()?;
    let top_row_definition = wrt::RowDefinition::new()?;
    let mid_row_def = wrt::RowDefinition::new()?;
    let bottom_row_def = wrt::RowDefinition::new()?;

    top_row_definition.set_height(wrt::GridLength {
        value: 1.0,
        grid_unit_type: wrt::GridUnitType::Auto,
    })?;
    bottom_row_def.set_height(wrt::GridLength {
        value: 1.0,
        grid_unit_type: wrt::GridUnitType::Auto,
    })?;
    grid.row_definitions()?.append(top_row_definition)?;
    grid.row_definitions()?.append(mid_row_def)?;
    grid.row_definitions()?.append(bottom_row_def)?;
    grid.column_definitions()?.append(column_definition)?;
    grid.set_background(create_color_brush(theme.white.clone())?)?;

    Ok(grid)
}

fn create_theme() -> winrt::Result<Theme> {
    let ui_settings = wrt::UISettings::new()?;
    let os_foreground = ui_settings.get_color_value(wrt::UIColorType::Foreground)?;
    let os_background = ui_settings.get_color_value(wrt::UIColorType::Background)?;
    let os_accent = ui_settings.get_color_value(wrt::UIColorType::Accent)?;
    let is_os_dark_mode = is_light_color(&os_foreground);

    let mut light_gray = os_foreground.clone();
    let mut dark_gray = os_foreground.clone();
    let white = os_background;
    let black = os_foreground;
    if is_os_dark_mode {
        light_gray.a = 20;
        dark_gray.a = 140;
    } else {
        light_gray.a = 20;
        dark_gray.a = 200;
    };

    Ok(Theme {
        white,
        black,
        light_gray,
        dark_gray,
        accent: os_accent,
    })
}

fn create_color_brush(color: wrt::Color) -> winrt::Result<wrt::SolidColorBrush> {
    let brush = wrt::SolidColorBrush::new()?;
    brush.set_color(color)?;

    Ok(brush)
}

fn is_light_color(color: &wrt::Color) -> bool {
    ((5 * (color.g as u32)) + (2 * (color.r as u32)) + (color.b as u32)) as u32 > ((8 * 128) as u32)
}

pub fn create_list_item(
    title: &str,
    subtext: &str,
    image: &wrt::Image,
    tag: &str,
    theme: &Theme,
) -> winrt::Result<wrt::UIElement> {
    let list_item_margins = wrt::Thickness {
        top: 5.,
        left: 15.,
        right: 15.,
        bottom: 5.,
    };
    let root_stack_panel = create_stack_panel()?;
    root_stack_panel.set_orientation(wrt::Orientation::Horizontal)?;

    let name_version_stack_panel = create_stack_panel()?;
    name_version_stack_panel.set_margin(&list_item_margins)?;

    let title_block = wrt::TextBlock::new()?;
    title_block.set_text(title)?;

    let subtitle_block = wrt::TextBlock::new()?;
    subtitle_block.set_text(subtext)?;
    subtitle_block.set_foreground(create_color_brush(theme.dark_gray.clone())?)?;

    name_version_stack_panel.children()?.append(title_block)?;
    name_version_stack_panel
        .children()?
        .append(subtitle_block)?;
    root_stack_panel.children()?.append(image)?;
    root_stack_panel
        .children()?
        .append(name_version_stack_panel)?;
    ui_element_set_string_tag(&root_stack_panel, tag).unwrap();

    Ok(root_stack_panel.into())
}

pub fn create_stack_panel() -> winrt::Result<wrt::StackPanel> {
    let stack_panel = winrt::factory::<wrt::StackPanel, wrt::IStackPanelFactory>()?
        .create_instance(winrt::Object::default(), &mut winrt::Object::default())?;

    Ok(stack_panel)
}

pub fn create_list<T: Clone>(
    list: &Vec<ListItem<T>>,
    theme: &Theme,
) -> winrt::Result<wrt::UIElement> {
    let list_control = winrt::factory::<wrt::ListView, wrt::IListViewFactory>()?
        .create_instance(winrt::Object::default(), &mut winrt::Object::default())?;
    list_control.set_margin(wrt::Thickness {
        top: 15.,
        left: 0.,
        right: 0.,
        bottom: 0.,
    })?;
    list_control.set_selection_mode(wrt::ListViewSelectionMode::Single)?;
    list_control.set_vertical_alignment(wrt::VerticalAlignment::Stretch)?;
    list_control.set_background(create_color_brush(theme.light_gray.clone())?)?;

    set_listview_items(&list_control, list, theme)?;
    list_control.set_selected_index(0)?;

    ui_element_set_string_tag(&list_control, LIST_CONTROL_NAME).unwrap();
    // ^-- .unwrap() is not consistent with the rest of error handling
    // however this is extremly unlikely to occur so a panic is OK here

    Ok(list_control.into())
}

pub fn set_listview_items<T: Clone>(
    list_control: &wrt::ListView,
    list: &[ListItem<T>],
    theme: &Theme,
) -> winrt::Result<()> {
    for item in list {
        list_control
            .items()?
            .append(winrt::Object::from(create_list_item(
                item.title.as_str(),
                item.subtitle.as_str(),
                &item.image,
                item.uuid.as_str(),
                &theme,
            )?))?;
    }

    Ok(())
}

pub fn create_header(
    open_action_text: &str,
    url: &str,
    theme: &Theme,
) -> winrt::Result<wrt::StackPanel> {
    let stack_panel = winrt::factory::<wrt::StackPanel, wrt::IStackPanelFactory>()?
        .create_instance(winrt::Object::default(), &mut winrt::Object::default())?;
    let call_to_action_top_row = wrt::TextBlock::new()?;
    let call_to_action_bottom_row = wrt::TextBlock::new()?;

    call_to_action_top_row.set_text(open_action_text)?;
    call_to_action_bottom_row.set_foreground(create_color_brush(theme.accent.clone())?)?;
    call_to_action_bottom_row.set_text(url)?;

    call_to_action_bottom_row.set_tag(wrt::PropertyValue::create_string(URL_CONTROL_NAME)?)?;
    stack_panel.set_tag(wrt::PropertyValue::create_string(HEADER_PANEL_NAME)?)?;

    stack_panel.children()?.append(call_to_action_top_row)?;
    stack_panel.children()?.append(call_to_action_bottom_row)?;
    stack_panel.set_margin(wrt::Thickness {
        left: 15.0,
        right: 15.0,
        top: 15.0,
        bottom: 0.0,
    })?;

    Ok(stack_panel)
}

/// From the given WinRT SoftwareBitmap it returns
/// the corresponding WinUI Image XAML control that can be inserted
/// as a node in any UIElement derived object
pub fn software_bitmap_to_xaml_image(bmp: wrt::SoftwareBitmap) -> winrt::Result<wrt::Image> {
    // ToDO: Can we achieve the same thing without this conversion?
    // Background: ImageSource.SetBitmapAsync will throw an exception if
    // the bitmap set is not Pixel Format: BGRA8, BitmapAlphaMode: Premulitplied
    // Does it work setting these flags without any pixel conversion?
    let bgra8_bmp = match bmp.bitmap_pixel_format()? {
        wrt::BitmapPixelFormat::Bgra8 => wrt::SoftwareBitmap::convert_with_alpha(
            bmp,
            wrt::BitmapPixelFormat::Bgra8,
            wrt::BitmapAlphaMode::Premultiplied,
        )?,
        _ => bmp,
    };

    let image_control = wrt::Image::new()?;
    let img_src: wrt::SoftwareBitmapSource = wrt::SoftwareBitmapSource::new()?;
    img_src.set_bitmap_async(bgra8_bmp)?;
    image_control.set_source(wrt::ImageSource::from(img_src))?;

    return Ok(image_control);
}

/// Converts a HICON to a SoftwareBitmap that can be used with WinUI controls
///
/// Notes:
/// - There probably is a simpler way to achieve this
/// - The function does not implement all possiblities described in the Windows API doc
/// thus it is possible that it might not work for certain icon formats
pub fn hicon_to_software_bitmap(hicon: winapi::HICON) -> BSResult<wrt::SoftwareBitmap> {
    let mut icon_info: winapi::ICONINFO = unsafe { MaybeUninit::uninit().assume_init() };
    let icon_result = unsafe { winapi::GetIconInfo(hicon, &mut icon_info) };
    if icon_result == 0 {
        bail!("Couldn't get icon info for HICON {:?}", hicon);
    }

    let dib_struct_size = std::mem::size_of::<winapi::DIBSECTION>()
        .try_into()
        .unwrap_or(0);
    let bitmap_struct_size = std::mem::size_of::<winapi::BITMAP>()
        .try_into()
        .unwrap_or(0);

    let mut dib: winapi::DIBSECTION = unsafe { MaybeUninit::uninit().assume_init() };
    let bytes_read = unsafe {
        winapi::GetObjectW(
            icon_info.hbmColor as *mut _ as *mut winapi::c_void,
            dib_struct_size,
            &mut dib as *mut _ as *mut winapi::c_void,
        )
    };

    if bytes_read == 0 {
        unsafe {
            winapi::DeleteObject(icon_info.hbmColor as winapi::HGDIOBJ);
            winapi::DeleteObject(icon_info.hbmMask as winapi::HGDIOBJ);
        }

        bail!("Error: winapi::GetObject returned 0 on ICONINFO.hbmColor bitmap.");
    }

    // BITMAP size is 32 bytes
    // DIBSECTION is 104 bytes
    let bmp_size_in_bytes =
        (dib.dsBm.bmHeight * dib.dsBm.bmWidth) * (dib.dsBm.bmBitsPixel as i32 / 8);

    let pixel_bytes_result = match bytes_read {
        bytes_read if bytes_read == bitmap_struct_size => {
            // when GetObject returns the size of the BITMAP structure
            // then dib.dsBm is a device dependent bitmap we need to use GetBitmapBits
            let mut img_bytes = Vec::<u8>::new();
            img_bytes.resize(bmp_size_in_bytes as usize, 0);

            let img_bytes_read = unsafe {
                winapi::GetBitmapBits(
                    icon_info.hbmColor,
                    bmp_size_in_bytes,
                    img_bytes.as_mut_slice().as_mut_ptr() as *mut winapi::c_void,
                )
            };
            // TODO: Replace GetBitmapBits with GetDibBits because GetBitmapBits is deprecated

            if img_bytes_read == 0 {
                Err("winapi::GetBitmapBits read 0 bytes from the ICONINFO.hbmColor")
            } else {
                Ok(img_bytes)
            }
        }
        bytes_read if bytes_read == dib_struct_size => {
            if dib.dsBm.bmBits as usize != 0 {
                Ok(unsafe {
                    std::slice::from_raw_parts::<u8>(
                        dib.dsBm.bmBits as *const u8,
                        bmp_size_in_bytes as usize,
                    )
                    .to_vec()
                })
            } else {
                Err("Unexpected NULL pointer for image bits from DIBSECTION.dsBm.bmBits")
            }
        }
        0 => Err("winapi::GetObject returned 0 on ICONINFO.hbmColor bitmap."),
        _ => Err(
            "Unexpected response from winapi::GetObject, was expecting read bytes \
            to match either the BITMAP struct size or the DIBSECTION struct size.",
        ),
    };

    let pixel_bytes = match pixel_bytes_result {
        Ok(bytes) => bytes,
        Err(error) => unsafe {
            winapi::DeleteObject(icon_info.hbmColor as winapi::HGDIOBJ);
            winapi::DeleteObject(icon_info.hbmMask as winapi::HGDIOBJ);
            bail!(error);
        },
    };

    let raw_pixels = pixel_bytes
        .chunks_exact(4)
        .map(|chunk| {
            u32::from_ne_bytes(
                chunk
                    .try_into()
                    .expect("Expected chunk size to be 4 bytes when converting to u32"),
            )
        })
        .collect::<Vec<u32>>();

    let data_writer = wrt::DataWriter::new()?;
    data_writer.write_bytes(as_u8_slice(&raw_pixels[..]))?;

    let i_buffer = data_writer.detach_buffer()?;
    let software_bitmap = wrt::SoftwareBitmap::create_copy_with_alpha_from_buffer(
        i_buffer,
        wrt::BitmapPixelFormat::Bgra8,
        dib.dsBm.bmWidth,
        dib.dsBm.bmHeight,
        wrt::BitmapAlphaMode::Straight,
    )?;
    // About the BitmapPixelFormat::Bgra8:
    // Hard coding pixel format to BGRA with 1 byte per color seems to work but it should be
    // detected since there are no guarantees the Windows API will always return this format

    unsafe {
        winapi::DeleteObject(icon_info.hbmColor as winapi::HGDIOBJ);
        winapi::DeleteObject(icon_info.hbmMask as winapi::HGDIOBJ);
    }

    return Ok(software_bitmap);
}

/// Centers the given [`Window`] on the monitor where the mouse cursor is found.
fn center_window_on_cursor_monitor(window: &Window) -> winrt::Result<()> {
    let cursor_pos = unsafe {
        let mut point: winapi::POINT = Default::default();
        winapi::GetCursorPos(std::ptr::addr_of_mut!(point));
        point
    };
    let current_monitor = window
        .available_monitors()
        .find(move |monitor| {
            let pos = monitor.position();
            let size = monitor.size();

            let is_below_top = cursor_pos.x >= pos.x && cursor_pos.y >= pos.y;
            let is_above_bottom = cursor_pos.x <= pos.x + size.width as i32
                && cursor_pos.y <= pos.y + size.height as i32;
            let is_within_monitor = is_below_top && is_above_bottom;

            is_within_monitor
        })
        .ok_or(winrt::Error::new(
            winrt::ErrorCode(1),
            "Could not determine monitor from the current cursor",
        ))?;

    let monitor_size = current_monitor.size();
    let monitor_pos = current_monitor.position();
    let window_size = window.outer_size();

    let window_center_x = (window_size.width / 2) as i32;
    let window_center_y = (window_size.height / 2) as i32;
    let monitor_center_x = (monitor_pos.x + (monitor_size.width as i32)) / 2;
    let monitor_center_y = (monitor_pos.y + (monitor_size.height as i32)) / 2;
    let window_x = monitor_center_x - window_center_x;
    let window_y = monitor_center_y - window_center_y;

    window.set_outer_position(winit::dpi::PhysicalPosition {
        x: window_x,
        y: window_y,
    });

    Ok(())
}

fn recursive_find_child_by_tag(
    parent: &impl winrt::ComInterface,
    needle: &str,
) -> winrt::Result<Option<wrt::UIElement>> {
    let items_control: wrt::Panel = parent.query();
    if items_control.is_null() {
        return Err(winrt::Error::new(
            winrt::ErrorCode(1),
            "Parent given could not be cast to WinUI Panel. Check the given control inherits from Panel."
        ));
    }

    let _items_count = items_control.children()?.size()?;
    let iterator = items_control.children()?.first()?;
    let mut element_found = wrt::UIElement::default();
    while iterator.has_current()? {
        let current = iterator.current()?;
        let child_of_current = match recursive_find_child_by_tag(&current, needle) {
            Ok(Some(found_child)) => found_child, // matches when a children of 'current' has the tag string equal to 'needle'
            _ => wrt::UIElement::default(), // matches when either error or no item was found, also applies when 'current' is not
                                            // a type of Control that inhertis from Panel (ie. the control does not have children)
        };

        if !child_of_current.is_null() {
            element_found = child_of_current;
            break;
        }

        let tag_value = match ui_element_get_tag_as_string(&current) {
            Ok(Some(string_value)) => string_value,
            _ => String::default(),
        };

        if tag_value == needle {
            element_found = current;
            break;
        }

        iterator.move_next()?;
    }

    match element_found.is_null() {
        true => Ok(None),
        false => Ok(Some(element_found)),
    }
}

fn ui_element_get_tag_as_string(el: &impl ComInterface) -> BSResult<Option<String>> {
    let item_as_framework_elem = ComInterface::query::<wrt::FrameworkElement>(el);
    if item_as_framework_elem.is_null() {
        bail!("Element given in not valid or does not inherit from FrameworkElement");
    }

    let tag: wrt::IPropertyValue = item_as_framework_elem.tag()?.query();
    if tag.is_null() {
        return Ok(None);
    }

    let tag_type = tag.r#type()?;
    if tag_type != wrt::PropertyType::String {
        bail!("Element tag is set but it is not a string.");
    }

    let value: String = tag.get_string()?.into();
    Ok(Some(value))
}

fn ui_element_set_string_tag(el: &impl ComInterface, tag: &str) -> BSResult<()> {
    let item_as_framework_elem = ComInterface::query::<wrt::FrameworkElement>(el);
    if item_as_framework_elem.is_null() {
        bail!("Element given in not valid or does not inherit from FrameworkElement");
    }

    item_as_framework_elem.set_tag(wrt::PropertyValue::create_string(tag)?)?;

    Ok(())
}
