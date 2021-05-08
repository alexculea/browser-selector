#[macro_use]
extern crate simple_error;

mod error;
mod os_util;
mod ui;

use ::std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::os_util::os_browsers;
use ui::{BrowserSelectorUI, UserInterface};

fn main() {
    std::panic::set_hook(Box::new(|panic_info: &std::panic::PanicInfo| {
        crate::os_util::output_panic_text(panic_info.to_string());
        std::process::exit(1);
    }));

    let app_name = env!("CARGO_PKG_NAME");
    let app_version = env!("CARGO_PKG_VERSION");
    let arguments: Vec<String> = std::env::args().collect();
    let first_cli_argument = if arguments.len() >= 2 {
        arguments[1].to_owned() // arg[0] is executable path    
    } else { 
        String::new() 
    };

    match first_cli_argument.as_str() {
        "--register" => os_browsers::register_as_system_browser(app_name, false),
        "--deregister" => os_browsers::register_as_system_browser(app_name, true)
    };

    let cli_arg_open_url = first_cli_argument;
    

    // let env_name = std::env::var("ENV").unwrap_or("production".to_string());
    // let config_dir = os_util::get_create_config_directory("browser-selector", &env_name).unwrap_or(
    //     std::env::current_dir()
    //     .unwrap()
    //     .to_owned()
    //     .as_os_str()
    //     .to_string_lossy()
    //     .to_string()
    // );
    // TODO: Reintroduce when config is being used

    let mut ui = BrowserSelectorUI::new().expect("Failed to initialize COM or WinUI");
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(format!("{} {}", app_name, app_version))
        .build(&event_loop)
        .unwrap();
    ui.create(&window)
        .expect("Failed to initialize WinUI XAML.");

    let browsers: Vec<os_browsers::Browser> =
        os_browsers::read_system_browsers_sync().expect("Could not read browser list");

    let list_items: Vec<ui::ListItem<os_browsers::Browser>> = browsers
        .iter()
        .map(ui_list_item_from_browser)
        .rev()
        .collect();

    ui.set_list(&list_items)
        .expect("Couldn't populate browsers in the UI.");
    ui.set_url(cli_arg_open_url.as_str())
        .expect("Couldn't render URL in the UI.");
    ui.on_list_item_selected(move |uuid| {
        if let Some(item) = list_items.iter().find(|item| item.uuid == uuid) {
            let mut command_arguments = item.state.arguments.clone();
            command_arguments.push(cli_arg_open_url.clone());

            std::process::Command::new(&item.state.exe_path)
                .args(command_arguments)
                .spawn()
                .expect(
                    format!("Couldn't run browser program at {}", &item.state.exe_path)
                        .to_owned()
                        .as_str(),
                );

            std::process::exit(0);
        }
    })
    .expect("Cannot set on click event handler.");

    // to load the UI from a xaml file instead:
    // use winrt::ComInterface;
    // use bindings::windows::ui::xaml::markup::XamlReader;
    // use bindings::windows::ui::xaml::UIElement;
    // let xaml = fs::read_to_string("src\\main.xaml").expect("Cant read XAML file");
    // let ui_container = XamlReader::load(xaml).expect("Failed loading XAML").query::<UIElement>();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(_size),
                ..
            } => {
                ui.update_layout_size(&window, &_size).unwrap();
                // this causes a memory violation
                // when the program is closed but does work correclty
                // while the program is running
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } if input.state == winit::event::ElementState::Pressed => {}
            _ => (),
        }
    });
}

fn ui_list_item_from_browser(browser: &os_browsers::Browser) -> ui::ListItem<os_browsers::Browser> {
    let image = BrowserSelectorUI::<os_browsers::Browser>::load_image(browser.exe_path.as_str())
        .unwrap_or_default();

    let uuid = {
        let mut hasher = DefaultHasher::new();
        browser.exe_path.hash(&mut hasher);
        hasher.finish().to_string()
    };

    ui::ListItem {
        title: browser.version.product_name.clone(),
        subtitle: vec![
            browser.version.product_version.clone(),
            browser.version.binary_type.to_string(),
            browser.version.company_name.clone(),
            browser.version.file_description.clone(),
        ]
        .into_iter()
        .filter(|itm| itm.len() > 0)
        .collect::<Vec<String>>()
        .join(" | "),
        image,
        uuid,
        state: std::rc::Rc::new(browser.clone()),
    }
}
