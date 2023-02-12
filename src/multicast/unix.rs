use std::{
    io::{self, IoSlice, IoSliceMut},
    mem,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    os::fd::{AsRawFd, RawFd},
};

use nix::sys::socket::{self, sockopt, ControlMessage, ControlMessageOwned, MsgFlags, SockaddrIn};
use tokio::{io::unix::AsyncFd, net::UdpSocket};

pub struct Message {
    pub data: Vec<u8>,
    pub sender: SocketAddrV4,
    pub interface: i32,
}

pub struct MulticastSocket {
    socket: AsyncFd<UdpSocket>,
}

impl MulticastSocket {
    pub async fn new(multicast_addr: Ipv4Addr, port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).await?;

        socket::setsockopt(socket.as_raw_fd(), sockopt::Ipv4PacketInfo, &true).map_err(Self::map_err)?;

        let interfaces = if_addrs::get_if_addrs()?;

        for interface in interfaces {
            if let IpAddr::V4(ip) = interface.addr.ip() {
                socket.join_multicast_v4(multicast_addr, ip)?;
            }
        }

        Ok(Self {
            socket: AsyncFd::new(socket)?,
        })
    }

    pub async fn read(&self) -> io::Result<Message> {
        loop {
            let mut guard = self.socket.readable().await?;

            match guard.try_io(|socket| Self::read_inner(socket.as_raw_fd())) {
                Ok(result) => return result,
                Err(_) => continue,
            }
        }
    }

    fn read_inner(fd: RawFd) -> io::Result<Message> {
        let mut buf = vec![0; 1024];
        let mut control_buffer = nix::cmsg_space!(libc::in_pktinfo);

        let msg = socket::recvmsg(fd, &mut [IoSliceMut::new(&mut buf)], Some(&mut control_buffer), MsgFlags::empty()).map_err(Self::map_err)?;

        let sender = msg.address.map(|x: SockaddrIn| x.into()).unwrap();

        let interface = msg.cmsgs().find_map(|cmsg| {
            if let ControlMessageOwned::Ipv4PacketInfo(pktinfo) = cmsg {
                Some(pktinfo.ipi_ifindex)
            } else {
                None
            }
        });

        Ok(Message {
            data: buf,
            sender,
            interface: interface.unwrap(),
        })
    }

    pub async fn write(&mut self, data: &[u8], interface: i32) -> io::Result<usize> {
        let dst_addr = if let SocketAddr::V4(x) = self.socket.get_ref().local_addr()? {
            x
        } else {
            panic!("unsupported")
        };

        self.write_to(data, interface, &dst_addr).await
    }

    pub async fn write_to(&mut self, data: &[u8], interface: i32, dst_addr: &SocketAddrV4) -> io::Result<usize> {
        loop {
            let mut guard = self.socket.writable().await?;

            match guard.try_io(|socket| Self::write_inner(socket.as_raw_fd(), data, interface, dst_addr)) {
                Ok(result) => return result,
                Err(_) => continue,
            }
        }
    }

    fn write_inner(fd: RawFd, data: &[u8], interface: i32, dst_addr: &SocketAddrV4) -> io::Result<usize> {
        let mut pkt_info: libc::in_pktinfo = unsafe { mem::zeroed() };
        pkt_info.ipi_ifindex = interface;

        let dst_addr = SockaddrIn::from(*dst_addr);

        socket::sendmsg(
            fd,
            &[IoSlice::new(data)],
            &[ControlMessage::Ipv4PacketInfo(&pkt_info)],
            MsgFlags::empty(),
            Some(&dst_addr),
        )
        .map_err(Self::map_err)
    }

    fn map_err(err: nix::Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err)
    }
}
