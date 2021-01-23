#![no_std]

extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};
use core::{marker::PhantomData, ptr::NonNull};
use either::{Either, Left, Right};
use itertools::unfold;
use windows_kernel_abstract::{
    log, mdl::LockedMdl, ErrorLike,
};
use windows_kernel_common_sys::{
    ntstatus, SynchronizationEvent, ADDRESS_FAMILY, AF_INET, IO_NO_INCREMENT,
    KEVENT, NTSTATUS, NULL, PDEVICE_OBJECT, PIRP, PVOID, SOCKADDR_IN, ULONG,
    USHORT, _KWAIT_REASON, _LOCK_OPERATION, _MODE,
};
use windows_kernel_ntoskrnl_sys::{
    IoAllocateIrp, IoAllocateMdl, IoFreeIrp, IoReuseIrp, KeInitializeEvent,
    KeResetEvent, KeSetEvent, KeWaitForSingleObject, IoSetCompletionRoutine,
};
use windows_kernel_netio_sys::{
    WskCaptureProviderNPI, WskDeregister, WskRegister, WskReleaseProviderNPI,
    PWSK_PROVIDER_BASIC_DISPATCH, PWSK_SOCKET, WSK_BUF, WSK_CLIENT_DISPATCH,
    WSK_CLIENT_NPI, WSK_FLAG_CONNECTION_SOCKET, WSK_NO_WAIT,
    WSK_PROVIDER_CONNECTION_DISPATCH, WSK_PROVIDER_NPI, WSK_REGISTRATION,
    WSK_SOCKET,
};

pub struct KernelSocketState<'a> {
    pub registration: WSK_REGISTRATION,
    pub wsk_provider_npi: WSK_PROVIDER_NPI,
    pub wsk_client_npi: WSK_CLIENT_NPI,
    _marker: PhantomData<&'a ()>,
}

pub struct KernelSocketAsyncContext {
    pub completion_event: KEVENT,
    pub irp: PIRP,
}

pub struct KernelSocket<'a: 'b, 'b> {
    wsk_socket: PWSK_SOCKET,
    wsk_connection_dispatch: Option<NonNull<WSK_PROVIDER_CONNECTION_DISPATCH>>,
    async_context: KernelSocketAsyncContext,
    _marker: PhantomData<&'b KernelSocketState<'a>>,
}

pub type Offset = u32;

pub enum KernelSocketTransfertType {
    Send,
    Receive,
}

#[derive(Clone, Copy)]
pub enum ReceiveState<T> {
    Complete(T),
    Partial(usize),
    Failure(NTSTATUS),
}

impl<'a> KernelSocketState<'a> {
    pub fn new(
        wsk_client_dispatch: &'a mut WSK_CLIENT_DISPATCH,
    ) -> Either<NTSTATUS, Box<Self>> {
        let mut state = Box::new(unsafe {
            KernelSocketState {
                registration: ::core::mem::zeroed(),
                wsk_provider_npi: ::core::mem::zeroed(),
                wsk_client_npi: WSK_CLIENT_NPI {
                    ClientContext: 0 as _,
                    Dispatch: wsk_client_dispatch,
                },
                _marker: PhantomData,
            }
        });
        match state.register() {
            Right(_) => match state.capture_provider_npi() {
                Right(_) => Right(state),
                Left(err) => Left(err),
            },
            Left(err) => Left(err),
        }
    }

    fn register(&mut self) -> Either<NTSTATUS, ()> {
        unsafe {
            WskRegister(&mut self.wsk_client_npi, &mut self.registration)
                .as_either()
        }
    }

    fn deregister(&mut self) {
        unsafe { WskDeregister(&mut self.registration) }
    }

    fn capture_provider_npi(&mut self) -> Either<NTSTATUS, ()> {
        unsafe {
            WskCaptureProviderNPI(
                &mut self.registration,
                WSK_NO_WAIT,
                &mut self.wsk_provider_npi,
            )
            .as_either()
        }
    }

    fn release_provider_npi(&mut self) {
        unsafe { WskReleaseProviderNPI(&mut self.registration) }
    }
}

impl<'a> Drop for KernelSocketState<'a> {
    fn drop(&mut self) {
        log!("Dropping kernel socket state...");
        self.release_provider_npi();
        log!("Dropped provider npi.");
        self.deregister();
        log!("Dropped registration.");
    }
}

impl KernelSocketAsyncContext {
    extern "C" fn completion_routine(
        _device_object: PDEVICE_OBJECT, _irp: PIRP, completion_event: PVOID,
    ) -> NTSTATUS {
        unsafe {
            KeSetEvent(completion_event as _, IO_NO_INCREMENT as _, 0);
        }
        ntstatus::STATUS_MORE_PROCESSING_REQUIRED
    }

    fn initialize(&mut self) -> Either<NTSTATUS, ()> {
        unsafe {
            KeInitializeEvent(
                &mut self.completion_event,
                SynchronizationEvent as _,
                0,
            );
            let irp = IoAllocateIrp(1, 0);
            if irp == 0 as _ {
                Left(ntstatus::STATUS_INSUFFICIENT_RESOURCES)
            } else {
                self.irp = irp;
                IoSetCompletionRoutine(
                    self.irp,
                    Some(KernelSocketAsyncContext::completion_routine),
                    core::mem::transmute(&mut self.completion_event),
                    true,
                    true,
                    true,
                );
                Right(())
            }
        }
    }

    fn wait_for_completion(
        &mut self, status: NTSTATUS,
    ) -> Either<NTSTATUS, ()> {
        match status {
            ntstatus::STATUS_PENDING => unsafe {
                KeWaitForSingleObject(
                    core::mem::transmute(&mut self.completion_event),
                    _KWAIT_REASON::Executive,
                    _MODE::KernelMode as _,
                    0,
                    0 as _,
                )
                .as_either()
            }
            .right_and_then(|_| {
                unsafe { (*self.irp).IoStatus.__bindgen_anon_1.Status }
                    .as_either()
            }),
            _ => status.as_either(),
        }
    }

    fn reset(&mut self) {
        unsafe {
            KeResetEvent(core::mem::transmute(&mut self.completion_event));
            IoReuseIrp(self.irp, ntstatus::STATUS_UNSUCCESSFUL);
            IoSetCompletionRoutine(
                self.irp,
                Some(KernelSocketAsyncContext::completion_routine),
                core::mem::transmute(&mut self.completion_event),
                true,
                true,
                true,
            );
        }
    }

    fn free(&self) { unsafe { IoFreeIrp(self.irp) } }
}

impl<'a, 'b> KernelSocket<'a, 'b> {
    pub fn new(
        kernel_socket_state: &'b mut KernelSocketState<'a>,
        address_family: ADDRESS_FAMILY, socket_type: USHORT, protocol: ULONG,
    ) -> Either<NTSTATUS, Box<Self>> {
        let mut socket = Box::new(KernelSocket {
            wsk_socket: 0 as _,
            wsk_connection_dispatch: None,
            async_context: KernelSocketAsyncContext {
                completion_event: unsafe { ::core::mem::zeroed() },
                irp: 0 as _,
            },
            _marker: PhantomData,
        });
        socket.async_context.initialize().right_and_then(|_| {
            unsafe { 
                kernel_socket_state
                    .wsk_provider_npi
                    .Dispatch
                    .as_ref()
                    .and_then(|dispatch| dispatch.WskSocket)
                    .map_or(Left(ntstatus::STATUS_NOT_FOUND), |wsk_socket_create| 
                        socket
                        .async_context
                        .wait_for_completion(
                            wsk_socket_create(
                                kernel_socket_state.wsk_provider_npi.Client,
                                address_family,
                                socket_type,
                                protocol,
                                WSK_FLAG_CONNECTION_SOCKET,
                                0 as _,
                                0 as _,
                                0 as _,
                                0 as _,
                                0 as _,
                                socket.async_context.irp,
                            )
                        )
                        .map_right(|_| {
                            let wsk_socket: *mut WSK_SOCKET =
                                ::core::mem::transmute(
                                    (*socket.async_context.irp)
                                        .IoStatus
                                        .Information,
                                );
                            socket.wsk_socket = wsk_socket;
                            socket.wsk_connection_dispatch =
                                core::mem::transmute((*wsk_socket).Dispatch);
                            socket
                        }))
            }
        })
    }

    fn close(&mut self) -> Either<NTSTATUS, ()> {
        self.wsk_connection_dispatch
            .as_ref()
            .map(|dispatch| unsafe { dispatch.as_ref() })
            .and_then(|dispatch| unsafe {
                let dispatch_ptr: PWSK_PROVIDER_BASIC_DISPATCH =
                    ::core::mem::transmute(&dispatch.__bindgen_padding_0);
                dispatch_ptr.as_ref()
            })
            .and_then(|basic_dispatch| basic_dispatch.WskCloseSocket)
            .map_or(
                ntstatus::STATUS_NOT_FOUND.as_either(),
                |wsk_close_socket| {
                    self.async_context.reset();
                    self.async_context.wait_for_completion(unsafe {
                        wsk_close_socket(
                            self.wsk_socket,
                            self.async_context.irp,
                        )
                    })
                },
            )
            .map_right(|_| self.async_context.free())
    }

    pub fn connect(
        &mut self, remote_address: &mut SOCKADDR_IN,
    ) -> Either<NTSTATUS, ()> {
        let mut local_address = SOCKADDR_IN {
            sin_family: AF_INET as _,
            sin_port: 0,
            sin_addr: Default::default(),
            sin_zero: Default::default(),
        };
        match self
            .wsk_connection_dispatch
            .as_ref()
            .map(|dispatch| unsafe { dispatch.as_ref() })
            .map(|dispatch| (dispatch.WskBind, dispatch.WskConnect))
        {
            Some((Some(wsk_bind), Some(wsk_connect))) => {
                self.async_context.reset();
                self.async_context
                    .wait_for_completion(unsafe {
                        wsk_bind(
                            self.wsk_socket,
                            core::mem::transmute(&mut local_address),
                            0,
                            self.async_context.irp,
                        )
                    })
                    .right_and_then(|_| {
                        self.async_context.reset();
                        self.async_context.wait_for_completion(unsafe {
                            wsk_connect(
                                self.wsk_socket,
                                core::mem::transmute(remote_address),
                                0,
                                self.async_context.irp,
                            )
                        })
                    })
            }
            _ => Left(ntstatus::STATUS_NOT_FOUND),
        }
    }

    pub fn transfert(
        &mut self, transfert_type: KernelSocketTransfertType, flags: ULONG,
        buffer: &mut [u8],
    ) -> Either<NTSTATUS, usize> {
        let buffer_length = buffer.len();
        unsafe {
            IoAllocateMdl(
                buffer.as_mut_ptr() as _,
                buffer.len() as _,
                0,
                0,
                NULL as _,
            )
            .as_mut()
            .map_or(Left(ntstatus::STATUS_INSUFFICIENT_RESOURCES), |mdl| {
                let mut wsk_buffer = WSK_BUF {
                    Offset: 0,
                    Length: buffer_length as _,
                    Mdl: mdl,
                };
                self.wsk_connection_dispatch
                    .as_ref()
                    .map(|dispatch| dispatch.as_ref())
                    .and_then(|dispatch| match transfert_type {
                        KernelSocketTransfertType::Send => dispatch.WskSend,
                        KernelSocketTransfertType::Receive => dispatch.WskReceive,
                    })
                    .map_or(
                        Left(ntstatus::STATUS_NOT_FOUND),
                        |transfert_function| {
                            self.async_context.reset();
                            LockedMdl::new(
                                mdl,
                                _MODE::KernelMode as _,
                                _LOCK_OPERATION::IoWriteAccess,
                            )
                            .right_and_then(|_| {
                                self.async_context.wait_for_completion(
                                    transfert_function(
                                        self.wsk_socket,
                                        &mut wsk_buffer,
                                        flags,
                                        self.async_context.irp,
                                    )
                                )
                            })
                        },
                    )
                    .right_and_then(|_| {
                        let receive_len =
                            (*self.async_context.irp).IoStatus.Information as _;
                        if receive_len == 0 {
                            Left(ntstatus::STATUS_END_OF_FILE)
                        } else {
                            Right(receive_len)
                        }
                    })
            })
        }
    }

    pub fn stream_receive<F, T: Copy>(
        &mut self, chunk_size: usize, callback: F,
    ) -> Option<ReceiveState<T>>
    where F: Fn(&[u8]) -> Option<ReceiveState<T>> {
        let mut receive_buffer: Vec<u8> = vec![0 as u8; chunk_size];
        unfold(ReceiveState::Partial(0), |state| {
            let next_state = match *state {
                ReceiveState::Partial(len) => {
                    if receive_buffer.len() == len {
                        receive_buffer.resize(len + chunk_size, 0);
                    }
                    match self.transfert(
                        KernelSocketTransfertType::Receive,
                        0,
                        &mut receive_buffer[len..],
                    ) {
                        Right(recv_len) => {
                            let recv_total_len = len + recv_len;
                            callback(&receive_buffer[..recv_total_len])
                        }
                        Left(err) => Some(ReceiveState::Failure(err)),
                    }
                }
                _ => None,
            };
            match next_state {
                Some(x) => {
                    *state = x;
                    next_state
                }
                None => next_state,
            }
        })
        .last()
    }
}

impl<'a, 'b> Drop for KernelSocket<'a, 'b> {
    fn drop(&mut self) {
        log!("Dropping socket, closing...");
        match self.close() {
            Right(_) => {
                log!("Socket closed successfully.");
            }
            Left(status) => {
                log!("Socket close failure: %X", status);
            }
        }
    }
}
