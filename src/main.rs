use async_abstractions::spawn_future;
use clone_all::clone_all;
use gio::prelude::*;
use gtk::prelude::*;
use std::env::args;
use std::sync::Mutex;
use tokio::runtime;
use web_view::*;

mod async_abstractions;
mod data;

lazy_static::lazy_static! {
    static ref DATA: Mutex<data::Data> = Mutex::new(data::Data::load().unwrap());
}

fn build_login_ui(
    window: &gtk::ApplicationWindow,
    runtime: runtime::Handle,
    initial_email: &str,
    initial_password: &str,
) {
    let login_vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let logo = gtk::Image::from_file("icon.svg");
    logo.set_halign(gtk::Align::Center);
    logo.set_margin_bottom(50);
    logo.set_margin_top(50);
    login_vbox.add(&logo);

    let email_ex = gtk::Label::new(Some("Email"));
    email_ex.set_halign(gtk::Align::Start);
    login_vbox.add(&email_ex);

    let email = gtk::Entry::new();
    email.set_text(initial_email);
    email.set_placeholder_text(Some("john.smith@mail.com"));
    email.set_activates_default(true);
    email.set_margin_bottom(25);
    login_vbox.add(&email);

    let pass_ex = gtk::Label::new(Some("Password"));
    pass_ex.set_halign(gtk::Align::Start);
    login_vbox.add(&pass_ex);

    let password = gtk::Entry::new();
    password.set_text(initial_password);
    password.set_activates_default(true);
    let invis_char = match password.get_invisible_char() {
        Some(c) => c,
        None => '*',
    };
    password.set_placeholder_text(Some(&invis_char.encode_utf8(&mut [0u8; 4]).repeat(16)));
    password.set_margin_bottom(25);
    password.set_visibility(false); // hide characters, because its a password
    login_vbox.add(&password);

    let button = gtk::Button::with_label("Login");

    button.connect_clicked({
        clone_all![window, login_vbox, email, password];
        move |_s| {
            let email_text = email.get_text().as_str().to_string();
            let password_text = password.get_text().as_str().to_string();

            // check if fields have been filled
            if email_text.len() == 0 {
                email.set_placeholder_text(Some("Please insert email!"));
                password.get_style_context().remove_class("login_error");
                email.get_style_context().add_class("login_error");
                return;
            }
            if password_text.len() == 0 {
                password.set_placeholder_text(Some("Please insert password!"));
                email.get_style_context().remove_class("login_error");
                password.get_style_context().add_class("login_error");
                return;
            }

            // change to spinning animation
            window.remove(&window.get_child().unwrap());
            build_waiting_ui(&window, runtime.clone(), "Logging in...");

            // try retrieving the token

            #[derive(serde::Serialize)]
            struct LoginData {
                email: String,
                password: String,
                undelete: bool,
                captcha_key: Option<()>,
                login_source: Option<()>,
                gift_code_sku_id: Option<()>,
            }

            spawn_future(
                runtime.clone(),
                {
                    clone_all![email_text, password_text];
                    async move {
                        let res = reqwest::Client::new()
                            .post("https://discord.com/api/v8/auth/login")
                            .json(&LoginData {
                                email: email_text,
                                password: password_text,
                                undelete: false,
                                captcha_key: None,
                                login_source: None,
                                gift_code_sku_id: None,
                            })
                            .send()
                            .await
                            .unwrap();

                        match res.json::<serde_json::Value>().await {
                            Ok(serde_json::Value::Object(o)) => {
                                let token = if o.contains_key("captcha_key") {
                                    // oh no looks like it's requiring a captcha to be completed
                                    match extract_token_from_discord() {
                                        Some(token) => token,
                                        None => {
                                            // looks like the user just closed the discord window :/
                                            // just exit
                                            std::process::exit(0);
                                        }
                                    }
                                } else if o.contains_key("token") {
                                    o["token"]
                                        .as_str()
                                        .expect("oopsie woopsie why is the token not a string??")
                                        .to_string()
                                } else if o.contains_key("errors") {
                                    return Err("Incorrect login info");
                                } else {
                                    eprintln!("Unknown login response: {:?}", o);
                                    std::process::exit(1);
                                };
                                let mut data_lock = DATA.lock().unwrap();
                                data_lock.discord_token = Some(token);
                                data_lock.save().unwrap();
                                drop(data_lock);
                            }
                            Err(e) => {
                                eprintln!("Error: {:?}", e);
                                std::process::exit(1);
                            }
                            Ok(d) => {
                                eprintln!("Unknown login response structure: {:?}", d);
                                std::process::exit(1);
                            }
                        };
                        Ok(())
                    }
                },
                Some({
                    clone_all![window, runtime];
                    move |res| {
                        window.remove(&window.get_child().unwrap());
                        match res {
                            Ok(()) => build_waiting_ui(&window, runtime.clone(), "Loading..."),
                            Err(e) => {
                                build_login_ui(
                                    &window,
                                    runtime.clone(),
                                    &email_text,
                                    &password_text,
                                );
                                let message = gtk::MessageDialog::new(
                                    Some(&window),
                                    gtk::DialogFlags::MODAL & gtk::DialogFlags::DESTROY_WITH_PARENT,
                                    gtk::MessageType::Error,
                                    gtk::ButtonsType::Ok,
                                    e,
                                );
                                message.run();
                                message.close();
                            }
                        }
                    }
                }),
            )
        }
    });
    login_vbox.add(&button);

    window.add(&login_vbox);
    window.show_all();

    button.set_can_default(true);
    button.set_property_has_default(true);
    button.grab_default();
}

fn build_waiting_ui(window: &gtk::ApplicationWindow, _runtime: runtime::Handle, text: &str) {
    // The spinning icon page

    let waiting_vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    waiting_vbox.set_valign(gtk::Align::Center);

    let waiting_label = gtk::Label::new(Some(text));
    waiting_label.set_halign(gtk::Align::Center);
    waiting_label.set_margin_top(25);
    waiting_label.set_margin_bottom(25);
    waiting_vbox.add(&waiting_label);

    let spinner = gtk::Spinner::new();
    spinner.set_halign(gtk::Align::Center);
    spinner.set_margin_bottom(25);
    spinner.start();
    waiting_vbox.add(&spinner);

    window.add(&waiting_vbox);
    window.show_all();
}

fn main() {
    lazy_static::initialize(&DATA);
    println!("{:?}", *DATA);

    let runtime = {
        let (sender, receiver) = std::sync::mpsc::sync_channel(0);
        std::thread::spawn(move || {
            let mut runtime = tokio::runtime::Builder::new()
                .enable_all()
                .basic_scheduler()
                .core_threads(1)
                .max_threads(1)
                .build()
                .unwrap();

            sender.send(runtime.handle().clone()).unwrap();

            runtime.block_on(futures::future::pending::<()>());
        });

        receiver.recv().unwrap()
    };

    let application = gtk::Application::new(Some("oxycord.oxycord"), Default::default())
        .expect("GTK application initialization failed.");

    application.connect_activate(move |app| {
        // initialize CSS
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(include_bytes!("style.css"))
            .expect("Failed to load CSS");
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );

        let window = gtk::ApplicationWindow::new(app);
        window.set_title("Oxycord Login");
        window
            .set_icon_from_file("icon.svg")
            .expect("failed to load icon");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 0);

        match &DATA.lock().unwrap().discord_token {
            Some(_token) => {
                build_waiting_ui(&window, runtime.clone(), "Loading...");
                // try connecting
            }
            None => build_login_ui(&window, runtime.clone(), "", ""),
        }
    });

    application.run(&args().collect::<Vec<_>>());
}

fn extract_token_from_discord() -> Option<String> {
    let mut token = None;
    let mut webview = WebViewBuilder::new()
        .title("Discord login")
        .content(Content::Url("https://discord.com/login"))
        .size(800, 600)
        .resizable(true)
        .user_data(())
        .invoke_handler(|webview, arg| {
            token = Some(arg.to_string());
            webview.exit();
            Ok(())
        })
        .build()
        .unwrap();
    webview.eval(include_str!("extract_token.js")).unwrap();
    webview.run().unwrap();

    token
}
