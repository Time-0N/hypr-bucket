use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use gtk4::Application;
use gtk4::glib;
use gtk4::prelude::{GtkApplicationExt, GtkWindowExt};

fn get_socket_path() -> PathBuf {
    let xdg_runtime = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/tmp/hypr-bucket-{}", unsafe { libc::getuid() }));

    let mut path = PathBuf::from(xdg_runtime);
    path.push("hyprbucket");
    path
}

pub fn notify_existing_instance() -> bool {
    use std::os::unix::net::UnixStream;

    let mut socket_path = get_socket_path();
    socket_path.push("hyprbucket.sock");

    if let Ok(_stream) = UnixStream::connect(&socket_path) {
        return true;
    }

    false
}

pub fn setup_socket_listener(app: Arc<Mutex<Option<Application>>>) {
    use std::os::unix::net::UnixListener;

    let socket_dir = get_socket_path();
    let socket_path = {
        let mut p = socket_dir.clone();
        p.push("hyprbucket.sock");
        p
    };

    let _ = fs::create_dir_all(&socket_dir);
    let _ = fs::remove_file(&socket_path);

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind socket: {}", e);
            return;
        }
    };

    let _ = listener.set_nonblocking(true);
    let fd = listener.as_raw_fd();

    glib::unix_fd_add_local(fd, glib::IOCondition::IN, move |_fd, condition| {
        if condition.contains(glib::IOCondition::IN) {
            if let Ok((stream, _)) = listener.accept() {
                drop(stream);
                if let Ok(app_lock) = app.lock() {
                    if let Some(app) = app_lock.as_ref() {
                        app.active_window().map(|w| w.close());
                    }
                }
            }
        }
        glib::ControlFlow::Continue
    });
}
