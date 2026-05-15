use adw::prelude::*;
use anonsurf_core::{CommandOutcome, Status, TorCheck, DBUS_INTERFACE, DBUS_PATH, DBUS_SERVICE};
use gtk::{gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use zbus::{Connection, Proxy};

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("org.anonsurf.rs1")
        .flags(gio::ApplicationFlags::empty())
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("AnonSurf")
        .default_width(860)
        .default_height(620)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    root.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    root.append(&content);

    let status_group = adw::PreferencesGroup::builder().title("Status").build();
    let status_row = adw::ActionRow::builder()
        .title("AnonSurf")
        .subtitle("Loading")
        .build();
    let tor_row = adw::ActionRow::builder()
        .title("Tor")
        .subtitle("Unknown")
        .build();
    let ip_row = adw::ActionRow::builder()
        .title("Current exit IP")
        .subtitle("Unknown")
        .build();
    let dns_row = adw::ActionRow::builder()
        .title("DNS mode")
        .subtitle("Unknown")
        .build();
    let fw_row = adw::ActionRow::builder()
        .title("Firewall backend")
        .subtitle("Unknown")
        .build();
    let bridge_row = adw::ActionRow::builder()
        .title("Bridge mode")
        .subtitle("Unknown")
        .build();
    status_group.add(&status_row);
    status_group.add(&tor_row);
    status_group.add(&ip_row);
    status_group.add(&dns_row);
    status_group.add(&fw_row);
    status_group.add(&bridge_row);
    content.append(&status_group);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::Start);
    let start = gtk::Button::with_label("Start");
    let stop = gtk::Button::with_label("Stop");
    let restart = gtk::Button::with_label("Restart");
    let new_identity = gtk::Button::with_label("New Identity");
    let tor_check = gtk::Button::with_label("Tor Check");
    let repair = gtk::Button::with_label("Repair Networking");
    repair.add_css_class("destructive-action");
    for button in [&start, &stop, &restart, &new_identity, &tor_check, &repair] {
        actions.append(button);
    }
    content.append(&actions);

    let logs_group = adw::PreferencesGroup::builder().title("Logs").build();
    let scroller = gtk::ScrolledWindow::builder()
        .min_content_height(220)
        .vexpand(true)
        .build();
    let logs = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    scroller.set_child(Some(&logs));
    logs_group.add(&scroller);
    content.append(&logs_group);

    let ui = Rc::new(RefCell::new(Ui {
        status_row,
        tor_row,
        ip_row,
        dns_row,
        fw_row,
        bridge_row,
        logs,
    }));

    {
        let ui = ui.clone();
        refresh(&ui);
    }

    wire_action(&start, "Start", ui.clone());
    wire_action(&stop, "Stop", ui.clone());
    wire_action(&restart, "Restart", ui.clone());
    wire_action(&new_identity, "NewIdentity", ui.clone());
    wire_tor_check(&tor_check, ui.clone());
    wire_action(&repair, "RepairNetworking", ui.clone());

    window.set_content(Some(&root));
    window.present();
}

struct Ui {
    status_row: adw::ActionRow,
    tor_row: adw::ActionRow,
    ip_row: adw::ActionRow,
    dns_row: adw::ActionRow,
    fw_row: adw::ActionRow,
    bridge_row: adw::ActionRow,
    logs: gtk::TextView,
}

fn wire_action(button: &gtk::Button, method: &'static str, ui: Rc<RefCell<Ui>>) {
    button.connect_clicked(move |_| {
        let reply = daemon_call(method, None).unwrap_or_else(|err| {
            format!(
                r#"{{"ok":false,"message":"{}","changed":[],"status":{}}}"#,
                json_escape(&err.to_string()),
                serde_json::to_string(&Status::default()).unwrap()
            )
        });
        append_log(&ui, &reply);
        if let Ok(outcome) = serde_json::from_str::<CommandOutcome>(&reply) {
            apply_status(&ui, &outcome.status);
        }
        refresh_logs(&ui);
    });
}

fn wire_tor_check(button: &gtk::Button, ui: Rc<RefCell<Ui>>) {
    button.connect_clicked(move |_| {
        let reply = daemon_call("TorCheck", None).unwrap_or_else(|err| {
            format!(
                r#"{{"ip":null,"is_tor":false,"source":"check.torproject.org","error":"{}"}}"#,
                json_escape(&err.to_string())
            )
        });
        append_log(&ui, &reply);
        if let Ok(check) = serde_json::from_str::<TorCheck>(&reply) {
            let ui = ui.borrow();
            ui.ip_row
                .set_subtitle(&check.ip.unwrap_or_else(|| "Unknown".to_string()));
            ui.status_row.set_subtitle(if check.is_tor {
                "Enabled / Tor verified"
            } else {
                "Tor check failed"
            });
        }
        refresh_logs(&ui);
    });
}

fn refresh(ui: &Rc<RefCell<Ui>>) {
    if let Ok(raw) = daemon_call("GetStatus", None) {
        if let Ok(status) = serde_json::from_str::<Status>(&raw) {
            apply_status(ui, &status);
        }
    }
    refresh_logs(ui);
}

fn refresh_logs(ui: &Rc<RefCell<Ui>>) {
    if let Ok(raw) = daemon_call("GetLogs", Some(100)) {
        if let Ok(lines) = serde_json::from_str::<Vec<String>>(&raw) {
            ui.borrow().logs.buffer().set_text(&lines.join("\n"));
        }
    }
}

fn apply_status(ui: &Rc<RefCell<Ui>>, status: &Status) {
    let ui = ui.borrow();
    ui.status_row.set_subtitle(&format!("{:?}", status.status));
    ui.tor_row.set_subtitle(&format!("{:?}", status.tor_status));
    ui.ip_row
        .set_subtitle(status.current_exit_ip.as_deref().unwrap_or("Unknown"));
    ui.dns_row
        .set_subtitle(&format!("{:?}", status.dns_backend));
    ui.fw_row
        .set_subtitle(&format!("{:?}", status.firewall_backend));
    ui.bridge_row
        .set_subtitle(&format!("{:?}", status.bridge_mode));
}

fn append_log(ui: &Rc<RefCell<Ui>>, line: &str) {
    let buffer = ui.borrow().logs.buffer();
    let mut current = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string();
    if !current.is_empty() {
        current.push('\n');
    }
    current.push_str(line);
    buffer.set_text(&current);
}

fn daemon_call(method: &str, u32_arg: Option<u32>) -> anyhow::Result<String> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let connection = Connection::system().await?;
        let proxy = Proxy::new(&connection, DBUS_SERVICE, DBUS_PATH, DBUS_INTERFACE).await?;
        if let Some(value) = u32_arg {
            Ok(proxy.call(method, &(value)).await?)
        } else {
            Ok(proxy.call(method, &()).await?)
        }
    })
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
