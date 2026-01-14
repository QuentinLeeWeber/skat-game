use std::fs;

fn main() {
    fs::read_to_string("server.conf")
        .expect("server.conf does not exist. In client root should exists a server.conf file. With form: {SERVER_IP}:[SERVER_PORT}");

    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
}
