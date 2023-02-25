#![allow(non_upper_case_globals, non_snake_case)]

use std::mem::size_of_val;

use lazy_static::lazy_static;

use windows::{
    core::GUID,
    Win32::Networking::WinSock::{
        closesocket, socket, WSAIoctl, AF_INET, IPPROTO_UDP, LPFN_WSARECVMSG, LPFN_WSASENDMSG, SIO_GET_EXTENSION_FUNCTION_POINTER, SOCK_DGRAM,
    },
};

lazy_static! {
    pub static ref WSASendMsg: LPFN_WSASENDMSG = get_wsa_send_msg_ptr();
    pub static ref WSARecvMsg: LPFN_WSARECVMSG = get_wsa_recv_msg_ptr();
}

fn get_wsa_send_msg_ptr() -> LPFN_WSASENDMSG {
    let guid = GUID {
        // WSAID_WSASENDMSG
        data1: 0xa441e712,
        data2: 0x754f,
        data3: 0x43ca,
        data4: [0x84, 0xa7, 0x0d, 0xee, 0x44, 0xcf, 0x60, 0x6d],
    };
    let mut ret = 0usize;
    let mut bytes = 0;

    unsafe {
        let s = socket(AF_INET.0 as _, SOCK_DGRAM as _, IPPROTO_UDP.0 as _); // dummy socket

        WSAIoctl(
            s,
            SIO_GET_EXTENSION_FUNCTION_POINTER,
            Some(&guid as *const _ as *mut _),
            size_of_val(&guid) as u32,
            Some(&mut ret as *mut _ as *mut _),
            size_of_val(&ret) as u32,
            &mut bytes,
            None,
            None,
        );

        closesocket(s);

        Some(std::mem::transmute(ret))
    }
}

fn get_wsa_recv_msg_ptr() -> LPFN_WSARECVMSG {
    let guid = GUID {
        // WSAID_WSARECVMSG
        data1: 0xf689d7c8,
        data2: 0x6f1f,
        data3: 0x436b,
        data4: [0x8a, 0x53, 0xe5, 0x4f, 0xe3, 0x51, 0xc3, 0x22],
    };
    let mut ret = 0usize;
    let mut bytes = 0;

    unsafe {
        let s = socket(AF_INET.0 as _, SOCK_DGRAM as _, IPPROTO_UDP.0 as _); // dummy socket

        WSAIoctl(
            s,
            SIO_GET_EXTENSION_FUNCTION_POINTER,
            Some(&guid as *const _ as *mut _),
            size_of_val(&guid) as u32,
            Some(&mut ret as *mut _ as *mut _),
            size_of_val(&ret) as u32,
            &mut bytes,
            None,
            None,
        );

        closesocket(s);

        Some(std::mem::transmute(ret))
    }
}
