use std::{env, fs, path::PathBuf};

fn main() {
    let source = fs::read_to_string("../../native/ui/app.slint").expect("read shared Slint UI");
    let (head, _) = source
        .split_once("export component AppTray inherits SystemTrayIcon")
        .expect("find native tray in shared Slint UI");
    let web_ui = head
        .replace("text: \"数据目录\"", "text: \"云端数据\"")
        .replace("text: \"退出\"", "text: \"刷新\"");
    let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("web-app.slint");
    fs::write(&path, web_ui).expect("write web Slint UI");
    slint_build::compile(path).expect("compile web Slint UI");
    println!("cargo:rerun-if-changed=../../native/ui/app.slint");
}
