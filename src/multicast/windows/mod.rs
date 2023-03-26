mod wsa;

use std::{
    io,
    mem::{size_of, size_of_val, zeroed},
    net::{IpAddr, Ipv4Addr, SocketAddrV4, UdpSocket},
    os::windows::io::{AsRawSocket, FromRawSocket},
    ptr::null_mut,
    sync::Once,
};

use tokio::task;
use windows::{
    core::PSTR,
    Win32::Networking::WinSock::{
        bind, setsockopt, socket, WSAGetLastError, ADDRESS_FAMILY, AF_INET, CMSGHDR, IN_PKTINFO, IPPROTO_IP, IPPROTO_UDP, IP_PKTINFO, SOCKADDR_IN,
        SOCKET, SOCK_DGRAM, SOL_SOCKET, SO_REUSEADDR, WSABUF, WSAMSG,
    },
};

use wsa::{WSARecvMsg, WSASendMsg};

use super::{InterfaceType, Message};

pub struct MulticastSocket {
    socket: UdpSocket,
    address: SocketAddrV4,
}

fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let _ = UdpSocket::bind("0.0.0.0:0"); // to properly call wsastartup()
    });
}

impl MulticastSocket {
    pub async fn new(multicast_addr: Ipv4Addr, port: u16) -> io::Result<Self> {
        init();
        let socket = unsafe {
            let socket = socket(AF_INET.0 as _, SOCK_DGRAM as _, IPPROTO_UDP.0 as _);

            setsockopt(socket, IPPROTO_IP as _, IP_PKTINFO as _, Some(&[1]));
            setsockopt(socket, SOL_SOCKET as _, SO_REUSEADDR as _, Some(&[1]));

            let addr = SOCKADDR_IN {
                sin_family: ADDRESS_FAMILY(AF_INET.0 as _),
                sin_port: port.to_be(),
                sin_addr: Ipv4Addr::UNSPECIFIED.into(),
                sin_zero: [Default::default(); 8],
            };
            let r = bind(socket, &addr as *const SOCKADDR_IN as _, size_of_val(&addr) as i32);
            if r != 0 {
                return Err(io::Error::last_os_error());
            }

            UdpSocket::from_raw_socket(socket.0 as _)
        };

        let interfaces = if_addrs::get_if_addrs()?;

        for interface in interfaces {
            if let IpAddr::V4(ip) = interface.addr.ip() {
                socket.join_multicast_v4(&multicast_addr, &ip)?;
            }
        }

        Ok(Self {
            socket,
            address: SocketAddrV4::new(multicast_addr, port),
        })
    }

    pub async fn read(&self) -> io::Result<Message> {
        let socket = self.socket.as_raw_socket();

        let (r, data_buffer, origin_address, control_buffer, read_bytes) = task::spawn_blocking(move || unsafe {
            let mut data_buffer = vec![0; 1024];
            let mut origin_address = zeroed::<SOCKADDR_IN>();
            let mut control_buffer = [0; size_of::<CMSGHDR>() + size_of::<IN_PKTINFO>()];
            let mut read_bytes = 0;

            let mut data = WSABUF {
                buf: PSTR::from_raw(data_buffer.as_mut_ptr()),
                len: data_buffer.len() as _,
            };

            let control = WSABUF {
                buf: PSTR::from_raw(control_buffer.as_mut_ptr()),
                len: control_buffer.len() as _,
            };

            let mut wsa_msg = WSAMSG {
                name: &mut origin_address as *mut SOCKADDR_IN as _,
                namelen: size_of_val(&origin_address) as _,
                lpBuffers: &mut data,
                Control: control,
                dwBufferCount: 1,
                dwFlags: 0,
            };

            let r = (WSARecvMsg.unwrap())(SOCKET(socket as _), &mut wsa_msg, &mut read_bytes, null_mut(), None);

            (r, data_buffer, origin_address, control_buffer, read_bytes)
        })
        .await?;

        if r != 0 {
            let error = unsafe { WSAGetLastError() };
            return Err(io::Error::from_raw_os_error(error.0));
        }

        let sender = SocketAddrV4::new(origin_address.sin_addr.into(), origin_address.sin_port);

        let pktinfo = unsafe { &*(control_buffer[size_of::<CMSGHDR>()..].as_ptr() as *const IN_PKTINFO) };

        Ok(Message {
            data: data_buffer[..(read_bytes as usize)].into(),
            sender,
            interface: pktinfo.ipi_ifindex,
        })
    }

    pub async fn write(&mut self, data: &[u8], interface: InterfaceType) -> io::Result<usize> {
        let address = self.address;

        self.write_to(data, interface, &address).await
    }

    pub async fn write_to(&mut self, data: &[u8], interface: InterfaceType, dst_addr: &SocketAddrV4) -> io::Result<usize> {
        let mut data = WSABUF {
            buf: PSTR::from_raw(data.as_ptr() as *mut _),
            len: data.len() as _,
        };

        let mut control_buffer = [0; size_of::<CMSGHDR>() + size_of::<IN_PKTINFO>()];
        unsafe {
            *(control_buffer[..size_of::<CMSGHDR>()].as_ptr() as *mut CMSGHDR) = CMSGHDR {
                cmsg_len: size_of::<CMSGHDR>() + size_of::<IN_PKTINFO>(),
                cmsg_level: IPPROTO_IP as _,
                cmsg_type: IP_PKTINFO as _,
            };

            *(control_buffer[size_of::<CMSGHDR>()..].as_ptr() as *mut IN_PKTINFO) = IN_PKTINFO {
                ipi_addr: (*dst_addr.ip()).into(),
                ipi_ifindex: interface,
            }
        }

        let control = WSABUF {
            buf: PSTR::from_raw(control_buffer.as_mut_ptr()),
            len: control_buffer.len() as _,
        };

        let mut destination = SOCKADDR_IN {
            sin_family: ADDRESS_FAMILY(AF_INET.0 as _),
            sin_port: self.address.port().to_be(),
            sin_addr: (*self.address.ip()).into(),
            sin_zero: [Default::default(); 8],
        };

        let mut wsa_msg = WSAMSG {
            name: &mut destination as *mut _ as *mut _,
            namelen: size_of_val(&destination) as _,
            lpBuffers: &mut data,
            Control: control,
            dwBufferCount: 1,
            dwFlags: 0,
        };

        let mut sent_bytes = 0;
        let socket = self.socket.as_raw_socket();
        let r = unsafe { (WSASendMsg.unwrap())(SOCKET(socket as _), &mut wsa_msg, 0, &mut sent_bytes, null_mut(), None) };
        if r != 0 {
            let error = unsafe { WSAGetLastError() };
            return Err(io::Error::from_raw_os_error(error.0));
        }

        Ok(sent_bytes as _)
    }
}
