//! Desktop notifications.

use crate::history::Entry;

pub fn ip_changed(entry: &Entry) {
    let summary = format!("Public IP changed to {}", entry.ip);
    let mut body = match entry.previous {
        Some(prev) => format!("Was {prev}"),
        None => String::new(),
    };
    if let Some(intel) = &entry.intel {
        if let Some(place) = intel.place() {
            body.push_str(&format!("\n{place}"));
        }
        let flags = intel.flag_labels();
        if !flags.is_empty() {
            body.push_str(&format!("\nFlags: {}", flags.join(", ")));
        }
    }
    // Showing can block briefly (D-Bus / WinRT); never stall the monitor for it.
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new().appname("IPTrail").summary(&summary).body(&body).show();
    });
}
