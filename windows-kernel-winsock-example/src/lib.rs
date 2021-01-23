#![no_std]

extern crate alloc;

use alloc::string::ToString;
use core::{
    panic::PanicInfo,
    str::{from_utf8, FromStr},
};
use either::{Left, Right};
use httparse::Response;
use winapi::shared::{inaddr::IN_ADDR, mstcpip::RtlIpv4StringToAddressA};
use windows_kernel_abstract::{log, ErrorLike};
use windows_kernel_common_sys::{
    ntstatus, AF_INET, DRIVER_OBJECT, IPPROTO_TCP, NTSTATUS, PCSTR,
    PDRIVER_OBJECT, PUNICODE_STRING, SOCKADDR_IN, SOCK_STREAM, TRUE,
};
use windows_kernel_netio_sys::WSK_CLIENT_DISPATCH;
use windows_kernel_winsock::{
    KernelSocket, KernelSocketState, KernelSocketTransfertType, ReceiveState,
};

#[global_allocator]
static GLOBAL: kernel_alloc::KernelAlloc = kernel_alloc::KernelAlloc;

#[export_name = "_fltused"]
static _FLTUSED: i32 = 0;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn __CxxFrameHandler3(
    _: *mut u8, _: *mut u8, _: *mut u8, _: *mut u8,
) -> i32 {
    unimplemented!()
}

pub extern "C" fn driver_unload(_driver_object: *mut DRIVER_OBJECT) {
    log!("Unloading driver");
}

#[no_mangle]
pub extern "C" fn driver_entry(
    driver_object: PDRIVER_OBJECT, _registry_path: PUNICODE_STRING,
) -> NTSTATUS {
    log!("Loading driver");

    unsafe {
        driver_object.as_mut().map(|driver_object_ref| {
            driver_object_ref.DriverUnload = Some(driver_unload)
        });
    }

    #[allow(unused_unsafe)]
    let mut wsk_client_dispatch = WSK_CLIENT_DISPATCH {
        Version: (1 << 8) as u16 | (0 & 0xFF) as u16,
        Reserved: 0,
        WskClientEvent: None,
    };

    let result = KernelSocketState::new(&mut wsk_client_dispatch).map_right(|mut state| {
        log!("Creating connection socket...");
        KernelSocket::new(&mut state, AF_INET as _, SOCK_STREAM as _, IPPROTO_TCP).right_and_then(|mut socket| {
            log!("Creating addr from IP and port...");
            let mut addr: IN_ADDR = Default::default();
            let mut terminator: PCSTR = 0 as _;
            unsafe {
                let httpbin_dot_org_ip = "54.164.234.192";
                RtlIpv4StringToAddressA(
                    httpbin_dot_org_ip.as_ptr() as _,
                    TRUE as _,
                    &mut terminator,
                    &mut addr
                ).as_either()
            }
            .right_and_then(|_| {
                log!("Connecting to remote host...");
                let mut remote_address = SOCKADDR_IN {
                    sin_family: AF_INET as _,
                    sin_port: 80_u16.to_be(),
                    sin_addr: addr,
                    sin_zero: Default::default(),
                };
                socket.connect(&mut remote_address)
            })
            .right_and_then(|_| {
                log!("Connection succeeded, sending request...");
                let mut request_buffer = *b"GET /range/4096 HTTP/1.1\r\nHost: 54.164.234.192\r\n\r\n";
                socket.transfert(KernelSocketTransfertType::Send, 0, &mut request_buffer)
            })
            .right_and_then(|send_len| {
                log!("Send completed with %d", send_len);
                socket
                    .stream_receive(512, |buffer| {
                        let mut headers = [httparse::EMPTY_HEADER; 16];
                        let mut response = Response::new(&mut headers);
                        match response.parse(&buffer) {
                            Ok(httparse::Status::Complete(response_header_len)) => {
                                let content_length_header =
                                    response.headers.iter().find(|h| h.name == "Content-Length");
                                match content_length_header {
                                    Some(header) => match from_utf8(header.value).map(usize::from_str) {
                                        Ok(Ok(content_length_value)) => {
                                            log!("Content-Length: %d", content_length_value);
                                            if buffer.len() == response_header_len + content_length_value {
                                                log!("Full reponse.");
                                                log!("== Headers ==");
                                                response.headers.iter().for_each(|h| {
                                                    if *h != httparse::EMPTY_HEADER {
                                                        log!(
                                                            "%.*s: %.*s",
                                                            h.name.len(),
                                                            h.name.as_ptr(),
                                                            h.value.len(),
                                                            h.value.as_ptr()
                                                        );
                                                    }
                                                });
                                                Some(ReceiveState::Complete(()))
                                            } else {
                                                log!("Partial reponse body.");
                                                Some(ReceiveState::Partial(buffer.len()))
                                            }
                                        }
                                        _ => Some(ReceiveState::Failure(ntstatus::STATUS_INVALID_PARAMETER)),
                                    },
                                    None => Some(ReceiveState::Failure(ntstatus::STATUS_NOT_FOUND)),
                                }
                            }
                            Ok(httparse::Status::Partial) => {
                                log!("Incomplete response.");
                                Some(ReceiveState::Partial(buffer.len()))
                            }
                            Err(err) => {
                                log!("Failed to parse response: %s.", err.to_string().as_ptr());
                                Some(ReceiveState::Failure(ntstatus::STATUS_INVALID_MESSAGE))
                            }
                        }
                    })
                    .map_or(Left(ntstatus::STATUS_INVALID_DOMAIN_STATE), |st| match st {
                        ReceiveState::Complete(_) => Right(()),
                        ReceiveState::Failure(status) => Left(status),
                        _ => Left(ntstatus::STATUS_UNSUCCESSFUL),
                    })
            })
        })
    });
    match result {
        Right(Right(_)) => {
            log!("Kernel socket operations succeeded!");
        }
        Right(Left(status)) => {
            log!("Kernel socket connection failed: %X", status);
        }
        Left(status) => {
            log!("Kernel socket state creation failed: %X", status);
        }
    }

    ntstatus::STATUS_SUCCESS
}
