#![windows_subsystem = "windows"]
#[macro_use]
extern crate simple_error;

#[macro_use]
extern crate rusqlite;

mod conf;
mod data;
mod error;
mod os;
mod ui;

use core::cell::RefCell;
use crate::os::shared::ActiveWindowInfo;
use std::rc::Rc;
use winit::event_loop::ControlFlow;

use crate::os::sys_browsers;
use crate::os::sys_browsers::Browser;
use crate::ui::{BrowserSelectorUI, ListItem, UserInterface};

fn main() {
    std::panic::set_hook(Box::new(|panic_info: &std::panic::PanicInfo| {
        crate::os::output_panic_text(panic_info.to_string());
    }));

    let config = Rc::new(conf::read_config().unwrap_or_default());
    let app_name = env!("CARGO_PKG_NAME");
    let app_version = env!("CARGO_PKG_VERSION");
    let target_url = Rc::new(
        std::env::args()
            .nth(1)
            .unwrap_or(config.default_url.clone()),
    );
    let mut statistics_optional: Option<Rc<RefCell<data::Statistics>>> = None;

    let ui_ref = Rc::new(RefCell::new(
        BrowserSelectorUI::new().expect("Failed to initialize COM or WinUI"),
    ));
    let event_loop = ui::ev_loop::make_ev_loop();
    let mut browsers: Box<Vec<Browser>> = Box::new(Default::default());
    let mut src_app_opt: Option<ActiveWindowInfo> = None;

    if config.statistics {
        src_app_opt = Some(os::get_active_window_info());
        statistics_optional = Some(Rc::new(RefCell::new(data::Statistics::new())));
        let statistics_ref = statistics_optional.clone().unwrap();
        let mut statistics = statistics_ref.borrow_mut();
        let mut statistics_db_path = std::env::current_exe().unwrap_or_default();
        statistics_db_path.set_file_name("statistics.sqlite");
        statistics.set_db_path(&statistics_db_path);
        statistics.migrate_async(|res| res.expect("Migration failed"));
    }

    {
        let mut ui = ui_ref.borrow_mut();
        let title = format!("{} {}", app_name, app_version);
        ui.create(&title, &event_loop)
            .expect("Failed to create main UI.");
    }

    *browsers = sys_browsers::read_system_browsers_sync().expect("Could not read browser list");

    if let Some(stats) = statistics_optional.clone() {
        let mut statistics = stats.borrow_mut();
        let source = src_app_opt.clone().unwrap_or_default().exe_path;
        let start_time = std::time::Instant::now();
        let browsers = browsers.clone();
        let ui_ref = Rc::clone(&ui_ref);
        let selections = browsers
            .iter()
            .map(|browser| -> data::SelectionEntity {
                data::SelectionEntity {
                    id: None,
                    path: Some(browser.exe_path.clone()),
                    path_hash: Some(browser.get_hash()),
                }
            })
            .collect();

        ui_ref
            .borrow()
            .prediction_set_is_loading(true)
            .expect("Failed to set loading state for predictions.");
        statistics.update_selections(selections, |res| {
            res.expect(
                "Failed updating the browsers available on the system to the statistics database.",
            )
        });
        statistics.predict(source, &target_url, move |result| {
            if let Ok(predicted_list) = result.as_ref() {
                let duration = start_time.elapsed();
                let duration_msec = duration.as_millis();
                let duration_str = format!("{} ms", duration_msec);
                let list = predicted_list
                    .iter()
                    .take(2)
                    .filter_map(|item: &data::SelectionEntity| {
                        browsers.iter().find(|browser| {
                            browser.get_hash().as_str() == item.path_hash.as_ref().unwrap().as_str()
                        })
                    })
                    .map(|browser| browser.try_into().unwrap()) // TODO: try_into() is expensive
                    .collect::<Vec<ListItem<Browser>>>();

                ui_ref
                    .borrow_mut()
                    .prediction_set_state(&list.as_slice(), &duration_str)
                    .expect("Failed setting predicted state for the prediction section");
            } else {
                ui_ref
                    .borrow()
                    .prediction_set_is_loading(false)
                    .expect("Failed stopping loading state for predictions.");
            }
        });
    }

    let list_items: Vec<ListItem<Browser>> = browsers
        .iter()
        .filter(|browser| config.browser_is_not_hidden(&browser.name, &browser.exe_path))
        .filter_map(|item| item.try_into().ok())
        .collect();

    {
        let mut ui = ui_ref.borrow_mut();
        let open_url_clone = Rc::clone(&target_url);
        let ev_loop_proxy = event_loop.create_proxy();
        let statistics_ref = statistics_optional.clone();
        let src_app_clone = src_app_opt.clone();

        ui.set_list(&list_items)
            .expect("Couldn't populate browsers in the UI.");
        ui.set_url(&target_url)
            .expect("Couldn't render URL in the UI.");
        ui.on_browser_selected(move |uuid| {
            let source = src_app_clone.clone().unwrap_or_default().exe_path;
            list_items
                .iter()
                .find(|item| item.uuid == uuid)
                .and_then(|item| Some(item.state.as_ref()))
                .and_then::<std::rc::Rc<Browser>, _>(|browser| {
                    os::shared::spawn_browser_process(
                        &browser.exe_path,
                        browser.arguments.clone(),
                        &open_url_clone,
                    );

                    if let Some(stats) = statistics_ref.clone() {
                        let browser_hash = browser.get_hash();
                        stats.borrow_mut().save_choice(
                            source,
                            &open_url_clone,
                            &browser_hash,
                            &browser.exe_path,
                            |res| res.expect("Failed to save choice in statistics."),
                        );
                    }

                    None
                });
            ev_loop_proxy.send_event(ui::ev_loop::UserEvent::Close).ok();
        })
        .expect("Cannot set on click event handler.");

        ui.center_window_on_cursor_monitor();
        ui.set_main_window_visible(true);
    }
    // end of scope is needed as it drops ui, releases the mutable strong ref from ui_ref
    // to allow the UI to be borrowed in other places without panicking

    let worker = statistics_optional.clone();
    event_loop.run(ui::ev_loop::make_runner(
        ui_ref.clone(),
        move |control_flow| {
            if let Some(worker_ref) = &worker {
                let mut statistics = worker_ref.borrow_mut();
                statistics.tick();

                if *control_flow == ControlFlow::Exit {
                    let max_time_wait = std::time::Duration::from_millis(15_000);
                    let mut time_waited = std::time::Duration::from_millis(0);
                    statistics.stop();
                    // TODO: Refactor to use Condvar
                    while !statistics.is_finished() {
                        let dur = std::time::Duration::from_millis(10);
                        std::thread::sleep(dur);
                        time_waited += dur;
                        if max_time_wait < time_waited {
                            println!("Max time waiting for bg thread reached!");
                            break;
                        }
                    }

                    println!("Exited worker ref waiting procedure.");
                    *control_flow = ControlFlow::Exit
                }
            }

            #[cfg(target_os = "windows")]
            if *control_flow == ControlFlow::Exit {
                // TODO: Investigate why the process hangs when returning control to winit
                // or when existing gracefully with ExitProcess
                os::terminate_current_process()
            }
        },
    ));
}
