use gio::prelude::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env::args;
use std::rc::Rc;
use std::sync::Arc;
use tokio::prelude::*;
use tokio::runtime;
use tokio::runtime::Runtime;

mod data;

fn build_login_ui(window: &gtk::ApplicationWindow, runtime: runtime::Handle) {
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
    email.set_placeholder_text(Some("john.smith@mail.com"));
    email.set_activates_default(true);
    email.set_margin_bottom(25);
    login_vbox.add(&email);

    let pass_ex = gtk::Label::new(Some("Password"));
    pass_ex.set_halign(gtk::Align::Start);
    login_vbox.add(&pass_ex);

    let password = gtk::Entry::new();
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

    let window_clone = window.clone();
    let login_vbox_clone = login_vbox.clone();
    let email_clone = email.clone();
    let password_clone = password.clone();
    button.connect_clicked(move |_s| {
        // check if fields have been filled
        if email_clone.get_text_length() == 0 {
            email_clone.set_placeholder_text(Some("Please insert email!"));
            return;
        }
        if password_clone.get_text_length() == 0 {
            password_clone.set_placeholder_text(Some("Please insert password!"));
            return;
        }
        // change to spinning animation
        window_clone.remove(&login_vbox_clone);
        build_waiting_ui(&window_clone, runtime.clone());

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

        let email_text = email_clone.clone().get_text().as_str().to_string();
        let password_text = password_clone.clone().get_text().as_str().to_string();
        runtime.spawn(async move {
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

            match res.json::<serde_json::value::Value>().await {
                Ok(t) => println!("data: {:?}", t),
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            };
        });
    });
    login_vbox.add(&button);

    window.add(&login_vbox);
    window.show_all();

    button.set_can_default(true);
    button.set_property_has_default(true);
    button.grab_default();
}

fn build_waiting_ui(window: &gtk::ApplicationWindow, runtime: runtime::Handle) {
    // The spinning icon page

    let waiting_vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    waiting_vbox.set_valign(gtk::Align::Center);

    let waiting_label = gtk::Label::new(Some("Logging in..."));
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
    let data = Rc::new(RefCell::new(data::Data::load().unwrap()));
    println!("{:?}", data);

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

    let data_clone = data.clone();
    application.connect_activate(move |app| {
        let window = gtk::ApplicationWindow::new(app);
        window.set_title("Oxycord Login");
        window
            .set_icon_from_file("icon.svg")
            .expect("failed to load icon");
        window.set_border_width(10);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(350, 0);

        match &data_clone.borrow().discord_token {
            Some(token) => {
                build_waiting_ui(&window, runtime.clone());
                // try connecting
            }
            None => build_login_ui(&window, runtime.clone()),
        }
    });

    application.run(&args().collect::<Vec<_>>());
}
