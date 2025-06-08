#[cfg(target_os = "linux")]
#[cfg(not(target_os = "linux"))]
compile_error!("This crate only supports Linux");

#[cfg(target_os = "linux")]
pub mod download;
#[cfg(target_os = "linux")]
pub mod server;
#[cfg(target_os = "linux")]
pub mod server_path;
#[cfg(target_os = "linux")]
pub mod validate;

#[cfg(target_os = "linux")]
pub fn check_ubuntu() {
    if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
        if !os_release.contains("Ubuntu") {
            eprintln!(
                "Warning: This crate is recommended to be run on Ubuntu. Other Linux distributions may not work as expected, but probably will."
            );
        }
    }
}
