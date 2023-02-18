mod wsa;

use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4},
};

use super::{InterfaceType, Message};

pub struct MulticastSocket {}

impl MulticastSocket {
    pub async fn new(_multicast_addr: Ipv4Addr, _port: u16) -> io::Result<Self> {
        unimplemented!()
    }

    pub async fn read(&self) -> io::Result<Message> {
        unimplemented!()
    }

    pub async fn write(&mut self, _data: &[u8], _interface: InterfaceType) -> io::Result<usize> {
        unimplemented!()
    }

    pub async fn write_to(&mut self, _data: &[u8], _interface: InterfaceType, _dst_addr: &SocketAddrV4) -> io::Result<usize> {
        unimplemented!()
    }
}
