#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::MulticastSocket;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::MulticastSocket;

#[cfg(target_os = "linux")]
type InterfaceType = i32;
#[cfg(any(target_os = "macos", target_os = "windows"))]
type InterfaceType = u32;

pub struct Message {
    pub data: Vec<u8>,
    pub sender: std::net::SocketAddrV4,
    pub interface: InterfaceType,
}
