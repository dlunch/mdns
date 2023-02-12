#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::MulticastSocket;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::MulticastSocket;

#[cfg(not(target_os = "macos"))]
type InterfaceType = i32;
#[cfg(target_os = "macos")]
type InterfaceType = u32;

pub struct Message {
    pub data: Vec<u8>,
    pub sender: std::net::SocketAddrV4,
    pub interface: InterfaceType,
}
