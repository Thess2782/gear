// This file is part of Gear.

// Copyright (C) 2023-2024 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Syscall implementations generic over wasmi and sandbox backends.

use crate::{
    accessors::{
        Read, ReadAs, ReadDecoded, ReadDecodedSpecial, SyscallArg, SyscallValue, Write, WriteAs,
    },
    error::{
        ActorTerminationReason, BackendAllocSyscallError, BackendSyscallError, RunFallibleError,
        TrapExplanation, UndefinedTerminationReason, UnrecoverableExecutionError,
        UnrecoverableMemoryError,
    },
    memory::{
        BackendMemory, ExecutorMemory, MemoryAccessError, MemoryAccessIo, MemoryAccessRegistry,
    },
    runtime::{CallerWrap, MemoryAccessIoOption, MemoryAccessIoWrapper},
    state::HostState,
    BackendExternalities,
};
use alloc::{
    format,
    string::{String, ToString},
};
use blake2::{digest::typenum::U32, Blake2b, Digest};
use core::marker::PhantomData;
use gear_core::{
    buffer::{RuntimeBuffer, RuntimeBufferSizeError},
    costs::CostToken,
    env::DropPayloadLockBound,
    gas::CounterType,
    ids::{MessageId, ProgramId, ReservationId},
    message::{
        HandlePacket, InitPacket, MessageWaitedType, Payload, PayloadSizeError, ReplyPacket,
    },
    pages::WasmPage,
};
use gear_core_errors::{MessageError, ReplyCode, SignalCode};
use gear_sandbox::{AsContextExt, ReturnValue, Value};
use gear_sandbox_env::{HostError, WasmReturnValue};
use gear_wasm_instrument::SystemBreakCode;
use gsys::{
    BlockNumberWithHash, ErrorBytes, ErrorWithGas, ErrorWithHandle, ErrorWithHash,
    ErrorWithReplyCode, ErrorWithSignalCode, ErrorWithTwoHashes, Gas, Hash, HashWithValue,
    TwoHashesWithValue,
};

/// BLAKE2b-256 hasher state.
type Blake2b256 = Blake2b<U32>;

type BuildResult<Call, Caller> = Result<(Call, MemoryAccessIoOption<Caller>), HostError>;

/// Actually just wrapper around [`ReturnValue`] to implement conversions.
pub struct SyscallReturnValue(ReturnValue);

impl From<SyscallReturnValue> for ReturnValue {
    fn from(value: SyscallReturnValue) -> Self {
        value.0
    }
}

impl From<()> for SyscallReturnValue {
    fn from((): ()) -> Self {
        Self(ReturnValue::Unit)
    }
}

impl From<i32> for SyscallReturnValue {
    fn from(value: i32) -> Self {
        Self(ReturnValue::Value(Value::I32(value)))
    }
}

impl From<u32> for SyscallReturnValue {
    fn from(value: u32) -> Self {
        Self(ReturnValue::Value(Value::I32(value as i32)))
    }
}

pub(crate) trait SyscallContext: Sized + Copy {
    fn from_args(args: &[Value]) -> Result<(Self, &[Value]), HostError>;
}

impl SyscallContext for () {
    fn from_args(args: &[Value]) -> Result<(Self, &[Value]), HostError> {
        Ok(((), args))
    }
}

pub(crate) trait Syscall<Caller, T = ()> {
    type Context: SyscallContext;

    fn execute(
        self,
        caller: &mut CallerWrap<Caller>,
        io: MemoryAccessIoWrapper<Caller>,
        ctx: Self::Context,
    ) -> Result<(Gas, T), HostError>;
}

/// Trait is implemented for functions.
///
/// # Generics
/// `Args` is to make specialization based on function arguments
/// `Ext` and `Res` are for syscall itself (`Syscall<Ext, Res>`)
pub(crate) trait SyscallBuilder<Caller, Args: ?Sized, Res, Call>
where
    Call: Syscall<Caller, Res>,
{
    fn build(self, ctx: &mut CallerWrap<Caller>, args: &[Value]) -> BuildResult<Call, Caller>;
}

impl<Caller, Res, Call, Builder> SyscallBuilder<Caller, (), Res, Call> for Builder
where
    Builder: FnOnce() -> Call,
    Call: Syscall<Caller, Res>,
{
    fn build(self, _: &mut CallerWrap<Caller>, args: &[Value]) -> BuildResult<Call, Caller> {
        let _: [Value; 0] = args.try_into().map_err(|_| HostError)?;
        Ok(((self)(), None))
    }
}

impl<Caller, Res, Call, Builder> SyscallBuilder<Caller, [Value], Res, Call> for Builder
where
    Builder: for<'a> FnOnce(&'a [Value]) -> Call,
    Call: Syscall<Caller, Res>,
{
    fn build(self, _: &mut CallerWrap<Caller>, args: &[Value]) -> BuildResult<Call, Caller> {
        Ok(((self)(args), None))
    }
}

// implement [`SyscallBuilder`] for functions with different amount of arguments
macro_rules! impl_syscall_builder {
    ($($generic:ident),+) => {
        #[allow(non_snake_case)]
        impl<Caller, Ext, Res, Call, Builder, $($generic),+> SyscallBuilder<Caller, ($($generic,)+), Res, Call>
            for Builder
        where
            Builder: FnOnce($($generic),+) -> Call,
            Call: Syscall<Caller, Res>,
            Caller: AsContextExt<State = HostState<Ext, BackendMemory<ExecutorMemory>>>,
            Ext: BackendExternalities + 'static,
            $( $generic: SyscallArg, )+
        {
            fn build(self, caller: &mut CallerWrap<Caller>, args: &[Value]) -> BuildResult<Call, Caller> {
                let ARGS_AMOUNT: usize = 0 $( + $generic::REQUIRED_ARGS )+;

                if args.len() != ARGS_AMOUNT {
                    log::debug!(target: "syscalls", "Invalid arguments amount, required args: {}, actual args: {}", ARGS_AMOUNT, args.len());
                    return Err(HostError);
                }

                let mut registry: Option<MemoryAccessRegistry<Caller>> = None;

                let mut index = 0;
                $(
                    if $generic::REQUIRES_MEMORY_MANAGER && registry.is_none() {
                        registry = Some(MemoryAccessRegistry::default());
                    }

                    let args_count = $generic::REQUIRED_ARGS;
                    let args_slice = &args[index..index + args_count];
                    let $generic = $generic::new(&mut registry, args_slice).map_err(|_| HostError)?;
                    index += args_count;
                )+

                if(index != ARGS_AMOUNT) {
                    log::debug!(target: "syscalls", "Invalid index");
                    return Err(HostError);
                }

                if registry.is_some() {
                    let io = registry.unwrap().pre_process(caller);
                    Ok(((self)($($generic),+), Some(io)))
                } else {
                    Ok(((self)($($generic),+), None))
                }
            }
        }
    };
}

impl_syscall_builder!(A);
impl_syscall_builder!(A, B);
impl_syscall_builder!(A, B, C);
impl_syscall_builder!(A, B, C, D);
impl_syscall_builder!(A, B, C, D, E);
impl_syscall_builder!(A, B, C, D, E, F);
impl_syscall_builder!(A, B, C, D, E, F, G);

/// "raw" syscall without any argument parsing or without calling [`CallerWrap`] helper methods
struct RawSyscall<F>(F);

impl<F> RawSyscall<F> {
    fn new(f: F) -> Self {
        Self(f)
    }
}

impl<T, F, Caller> Syscall<Caller, T> for RawSyscall<F>
where
    F: FnOnce(&mut CallerWrap<Caller>) -> Result<(Gas, T), HostError>,
{
    type Context = ();

    fn execute(
        self,
        caller: &mut CallerWrap<Caller>,
        _io: MemoryAccessIoWrapper<Caller>,
        (): Self::Context,
    ) -> Result<(Gas, T), HostError> {
        (self.0)(caller)
    }
}

/// Fallible syscall context that parses `gas` and `err_ptr` arguments.
#[derive(Copy, Clone)]
struct FallibleSyscallContext {
    gas: Gas,
    res_ptr: u32,
}

impl SyscallContext for FallibleSyscallContext {
    fn from_args(args: &[Value]) -> Result<(Self, &[Value]), HostError> {
        let (gas, args) = args.split_first().ok_or(HostError)?;
        let gas: Gas = SyscallValue(*gas).try_into()?;
        let (res_ptr, args) = args.split_last().ok_or(HostError)?;
        let res_ptr: u32 = SyscallValue(*res_ptr).try_into()?;
        Ok((FallibleSyscallContext { gas, res_ptr }, args))
    }
}

/// Fallible syscall that calls [`CallerWrap::run_fallible`] underneath.
#[derive(Copy, Clone)]
struct FallibleSyscall<E, F> {
    token: CostToken,
    error: PhantomData<E>,
    f: F,
}

impl<F> FallibleSyscall<(), F> {
    fn new<E>(token: CostToken, f: F) -> FallibleSyscall<E, F> {
        FallibleSyscall {
            token,
            error: PhantomData,
            f,
        }
    }
}

impl<T, E, F, Caller, Ext> Syscall<Caller, ()> for FallibleSyscall<E, F>
where
    F: FnOnce(
        &mut CallerWrap<Caller>,
        &mut MemoryAccessIoWrapper<Caller>,
    ) -> Result<T, RunFallibleError>,
    E: From<Result<T, u32>>,
    Caller: AsContextExt<State = HostState<Ext, BackendMemory<ExecutorMemory>>>,
    Ext: BackendExternalities + 'static,
{
    type Context = FallibleSyscallContext;

    fn execute(
        self,
        caller: &mut CallerWrap<Caller>,
        io: MemoryAccessIoWrapper<Caller>,
        context: Self::Context,
    ) -> Result<(Gas, ()), HostError> {
        let Self { token, f, .. } = self;
        let FallibleSyscallContext { gas, res_ptr } = context;
        caller.run_fallible::<T, _, E>(io, gas, res_ptr, token, f)
    }
}

/// Infallible syscall context that parses `gas` argument.
#[derive(Copy, Clone)]
pub struct InfallibleSyscallContext {
    gas: Gas,
}

impl SyscallContext for InfallibleSyscallContext {
    fn from_args(args: &[Value]) -> Result<(Self, &[Value]), HostError> {
        let (gas, args) = args.split_first().ok_or(HostError)?;
        let gas: Gas = SyscallValue(*gas).try_into()?;
        Ok((Self { gas }, args))
    }
}

/// Infallible syscall that calls [`CallerWrap::run_any`] underneath
#[derive(Copy, Clone)]
struct InfallibleSyscall<F> {
    token: CostToken,
    f: F,
}

impl<F> InfallibleSyscall<F> {
    fn new(token: CostToken, f: F) -> Self {
        Self { token, f }
    }
}

impl<T, F, Caller, Ext> Syscall<Caller, T> for InfallibleSyscall<F>
where
    F: FnOnce(
        &mut CallerWrap<Caller>,
        &mut MemoryAccessIoWrapper<Caller>,
    ) -> Result<T, UndefinedTerminationReason>,
    Caller: AsContextExt<State = HostState<Ext, BackendMemory<ExecutorMemory>>>,
    Ext: BackendExternalities + 'static,
{
    type Context = InfallibleSyscallContext;

    fn execute(
        self,
        caller: &mut CallerWrap<Caller>,
        io: MemoryAccessIoWrapper<Caller>,
        ctx: Self::Context,
    ) -> Result<(Gas, T), HostError> {
        let Self { token, f } = self;
        let InfallibleSyscallContext { gas } = ctx;
        caller.run_any::<T, _>(io, gas, token, f)
    }
}

pub(crate) struct FuncsHandler<Caller> {
    _phantom: PhantomData<Caller>,
}

impl<Caller, Ext> FuncsHandler<Caller>
where
    Caller: AsContextExt<State = HostState<Ext, BackendMemory<ExecutorMemory>>>,
    Ext: BackendExternalities + 'static,
    Ext::UnrecoverableError: BackendSyscallError,
    RunFallibleError: From<Ext::FallibleError>,
    Ext::AllocError: BackendAllocSyscallError<ExtError = Ext::UnrecoverableError>,
{
    pub fn execute<Builder, Args, Res, Call>(
        caller: &mut Caller,
        args: &[Value],
        builder: Builder,
    ) -> Result<WasmReturnValue, HostError>
    where
        Builder: SyscallBuilder<Caller, Args, Res, Call>,
        Args: ?Sized,
        Call: Syscall<Caller, Res>,
        Res: Into<SyscallReturnValue>,
    {
        crate::log::trace_syscall::<Builder>(args);

        let mut caller = CallerWrap::new(caller);

        let (ctx, args) = Call::Context::from_args(args)?;

        let (syscall, io) = builder.build(&mut caller, args)?;

        let io = MemoryAccessIoWrapper::new(io);

        let (gas, value) = syscall.execute(&mut caller, io, ctx)?;
        let value = value.into();

        Ok(WasmReturnValue {
            gas: gas as i64,
            inner: value.0,
        })
    }

    fn read_message_payload(
        ctx: &mut CallerWrap<Caller>,
        io: &MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        payload: Read,
    ) -> Result<Payload, RunFallibleError> {
        payload
            .read(ctx, io)?
            .try_into()
            .map_err(|PayloadSizeError| MessageError::MaxMessageSizeExceed.into())
            .map_err(RunFallibleError::FallibleExt)
    }

    fn send_inner(
        ctx: &mut CallerWrap<Caller>,
        io: &MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        pid_value: ReadAs<HashWithValue>,
        payload: Read,
        gas_limit: Option<u64>,
        delay: u32,
    ) -> Result<MessageId, RunFallibleError> {
        let HashWithValue {
            hash: destination,
            value,
        } = pid_value.read(ctx, io)?;

        let payload = Self::read_message_payload(ctx, io, payload)?;

        ctx.ext_mut()
            .send(
                HandlePacket::maybe_with_gas(destination.into(), payload, gas_limit, value),
                delay,
            )
            .map_err(Into::into)
    }

    pub fn send(
        pid_value: ReadAs<HashWithValue>,
        payload: Read,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::Send(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::send_inner(ctx, io.io_mut_ref()?, pid_value, payload, None, delay)
            },
        )
    }

    pub fn send_wgas(
        pid_value: ReadAs<HashWithValue>,
        payload: Read,
        gas_limit: u64,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::SendWGas(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::send_inner(
                    ctx,
                    io.io_mut_ref()?,
                    pid_value,
                    payload,
                    Some(gas_limit),
                    delay,
                )
            },
        )
    }

    fn send_commit_inner(
        ctx: &mut CallerWrap<Caller>,
        io: &MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        handle: u32,
        pid_value: ReadAs<HashWithValue>,
        gas_limit: Option<u64>,
        delay: u32,
    ) -> Result<MessageId, RunFallibleError> {
        let HashWithValue {
            hash: destination,
            value,
        } = pid_value.read(ctx, io)?;

        ctx.ext_mut()
            .send_commit(
                handle,
                HandlePacket::maybe_with_gas(
                    destination.into(),
                    Default::default(),
                    gas_limit,
                    value,
                ),
                delay,
            )
            .map_err(Into::into)
    }

    pub fn send_commit(
        handle: u32,
        pid_value: ReadAs<HashWithValue>,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::SendCommit,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::send_commit_inner(ctx, io.io_mut_ref()?, handle, pid_value, None, delay)
            },
        )
    }

    pub fn send_commit_wgas(
        handle: u32,
        pid_value: ReadAs<HashWithValue>,
        gas_limit: u64,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::SendCommitWGas,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::send_commit_inner(
                    ctx,
                    io.io_mut_ref()?,
                    handle,
                    pid_value,
                    Some(gas_limit),
                    delay,
                )
            },
        )
    }

    pub fn send_init() -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHandle>(
            CostToken::SendInit,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut().send_init().map_err(Into::into)
            },
        )
    }

    pub fn send_push(handle: u32, payload: Read) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::SendPush(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let payload = payload.read(ctx, io.io_mut_ref()?)?;

                ctx.ext_mut()
                    .send_push(handle, &payload)
                    .map_err(Into::into)
            },
        )
    }

    pub fn reservation_send(
        rid_pid_value: ReadAs<TwoHashesWithValue>,
        payload: Read,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReservationSend(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let TwoHashesWithValue {
                    hash1: reservation_id,
                    hash2: destination,
                    value,
                } = rid_pid_value.read(ctx, io.io_mut_ref()?)?;
                let payload = Self::read_message_payload(ctx, io.io_mut_ref()?, payload)?;

                ctx.ext_mut()
                    .reservation_send(
                        reservation_id.into(),
                        HandlePacket::new(destination.into(), payload, value),
                        delay,
                    )
                    .map_err(Into::into)
            },
        )
    }

    pub fn reservation_send_commit(
        handle: u32,
        rid_pid_value: ReadAs<TwoHashesWithValue>,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReservationSendCommit,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let TwoHashesWithValue {
                    hash1: reservation_id,
                    hash2: destination,
                    value,
                } = rid_pid_value.read(ctx, io.io_mut_ref()?)?;

                ctx.ext_mut()
                    .reservation_send_commit(
                        reservation_id.into(),
                        handle,
                        HandlePacket::new(destination.into(), Default::default(), value),
                        delay,
                    )
                    .map_err(Into::into)
            },
        )
    }

    pub fn read(at: u32, buffer: Write) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::Read,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let payload_lock = ctx.ext_mut().lock_payload(at, buffer.size())?;
                payload_lock
                    .drop_with::<MemoryAccessError, _>(move |payload_access| {
                        let f = || {
                            buffer.write(ctx, io.io_mut_ref()?, payload_access.as_slice())?;
                            Ok(())
                        };
                        let res = f();
                        let unlock_bound = ctx.ext_mut().unlock_payload(payload_access.into_lock());

                        DropPayloadLockBound::from((unlock_bound, res))
                    })
                    .into_inner()
                    .map_err(Into::into)
            },
        )
    }

    pub fn size(size_write: WriteAs<[u8; 4]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Size,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let size = ctx.ext_mut().size()? as u32;

                size_write
                    .write(ctx, io.io_mut_ref()?, size.to_le_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn exit(inheritor_id: ReadDecoded<ProgramId>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Exit,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let inheritor_id = inheritor_id.read(ctx, io.io_mut_ref()?)?;
                Err(ActorTerminationReason::Exit(inheritor_id).into())
            },
        )
    }

    pub fn reply_code() -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithReplyCode>(
            CostToken::ReplyCode,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut()
                    .reply_code()
                    .map(ReplyCode::to_bytes)
                    .map_err(Into::into)
            },
        )
    }

    pub fn signal_code() -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithSignalCode>(
            CostToken::SignalCode,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut()
                    .signal_code()
                    .map(SignalCode::to_u32)
                    .map_err(Into::into)
            },
        )
    }

    pub fn alloc(pages: u32) -> impl Syscall<Caller, u32> {
        InfallibleSyscall::new(
            CostToken::Alloc,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                let res = ctx.alloc(pages);
                let res = ctx.process_alloc_func_result(res)?;

                let page = match res {
                    Ok(page) => {
                        log::trace!("Alloc {pages:?} pages at {page:?}");
                        page.into()
                    }
                    Err(err) => {
                        log::trace!("Alloc failed: {err}");
                        u32::MAX
                    }
                };
                Ok(page)
            },
        )
    }

    pub fn free(page_no: u32) -> impl Syscall<Caller, i32> {
        InfallibleSyscall::new(
            CostToken::Free,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                let page = WasmPage::try_from(page_no).map_err(|_| {
                    UndefinedTerminationReason::Actor(ActorTerminationReason::Trap(
                        TrapExplanation::Unknown,
                    ))
                })?;

                let res = ctx.ext_mut().free(page);
                let res = ctx.process_alloc_func_result(res)?;

                match &res {
                    Ok(()) => {
                        log::trace!("Free {page:?}");
                    }
                    Err(err) => {
                        log::trace!("Free failed: {err}");
                    }
                };

                Ok(res.is_err() as i32)
            },
        )
    }

    pub fn free_range(start: u32, end: u32) -> impl Syscall<Caller, i32> {
        InfallibleSyscall::new(
            CostToken::FreeRange,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                let page_err = |_| {
                    UndefinedTerminationReason::Actor(ActorTerminationReason::Trap(
                        TrapExplanation::Unknown,
                    ))
                };

                let start = WasmPage::try_from(start).map_err(page_err)?;
                let end = WasmPage::try_from(end).map_err(page_err)?;

                let result = ctx.ext_mut().free_range(start, end);

                match ctx.process_alloc_func_result(result)? {
                    Ok(()) => {
                        log::trace!("Free range {start:?}:{end:?} success");
                        Ok(0)
                    }
                    Err(e) => {
                        log::trace!("Free range {start:?}:{end:?} failed: {e}");
                        Ok(1)
                    }
                }
            },
        )
    }

    pub fn env_vars(vars_ver: u32, vars_ptr: u32) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::EnvVars,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                let vars = ctx.ext_mut().env_vars(vars_ver)?;
                let vars_bytes = vars.to_bytes();

                let mut registry = MemoryAccessRegistry::default();
                let vars_write = registry.register_write(vars_ptr, vars_bytes.len() as u32);
                let mut io = registry.pre_process(ctx)?;
                io.write(ctx, vars_write, vars_bytes).map_err(Into::into)
            },
        )
    }

    pub fn block_height(height_write: WriteAs<[u8; 4]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::BlockHeight,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let height = ctx.ext_mut().block_height()?;

                height_write
                    .write(ctx, io.io_mut_ref()?, height.to_le_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn block_timestamp(timestamp_write: WriteAs<[u8; 8]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::BlockTimestamp,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let timestamp = ctx.ext_mut().block_timestamp()?;

                timestamp_write
                    .write(ctx, io.io_mut_ref()?, timestamp.to_le_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn random(
        subject_ptr: ReadDecoded<Hash>,
        bn_random_ptr: WriteAs<BlockNumberWithHash>,
    ) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Random,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let raw_subject = subject_ptr.read(ctx, io.io_mut_ref()?)?;
                let (random, bn) = ctx.ext_mut().random()?;
                let subject = [&raw_subject, random].concat();

                let mut blake2_ctx = Blake2b256::new();
                blake2_ctx.update(subject);
                let hash = blake2_ctx.finalize().into();

                bn_random_ptr
                    .write(ctx, io.io_mut_ref()?, BlockNumberWithHash { bn, hash })
                    .map_err(Into::into)
            },
        )
    }

    fn reply_inner(
        ctx: &mut CallerWrap<Caller>,
        io: &mut MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        payload: Read,
        gas_limit: Option<u64>,
        value: ReadDecodedSpecial<u128>,
    ) -> Result<MessageId, RunFallibleError> {
        let value = value.read(ctx, io)?;
        let payload = Self::read_message_payload(ctx, io, payload)?;

        ctx.ext_mut()
            .reply(ReplyPacket::maybe_with_gas(payload, gas_limit, value))
            .map_err(Into::into)
    }

    pub fn reply(payload: Read, value: ReadDecodedSpecial<u128>) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::Reply(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::reply_inner(ctx, io.io_mut_ref()?, payload, None, value)
            },
        )
    }

    pub fn reply_wgas(
        payload: Read,
        gas_limit: u64,
        value: ReadDecodedSpecial<u128>,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReplyWGas(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::reply_inner(ctx, io.io_mut_ref()?, payload, Some(gas_limit), value)
            },
        )
    }

    fn reply_commit_inner(
        ctx: &mut CallerWrap<Caller>,
        io: &mut MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        gas_limit: Option<u64>,
        value: ReadDecodedSpecial<u128>,
    ) -> Result<MessageId, RunFallibleError> {
        let value = value.read(ctx, io)?;

        ctx.ext_mut()
            .reply_commit(ReplyPacket::maybe_with_gas(
                Default::default(),
                gas_limit,
                value,
            ))
            .map_err(Into::into)
    }

    pub fn reply_commit(value: ReadDecodedSpecial<u128>) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReplyCommit,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::reply_commit_inner(ctx, io.io_mut_ref()?, None, value)
            },
        )
    }

    pub fn reply_commit_wgas(
        gas_limit: u64,
        value: ReadDecodedSpecial<u128>,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReplyCommitWGas,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::reply_commit_inner(ctx, io.io_mut_ref()?, Some(gas_limit), value)
            },
        )
    }

    pub fn reservation_reply(
        rid_value: ReadAs<HashWithValue>,
        payload: Read,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReservationReply(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let HashWithValue {
                    hash: reservation_id,
                    value,
                } = rid_value.read(ctx, io.io_mut_ref()?)?;
                let payload = Self::read_message_payload(ctx, io.io_mut_ref()?, payload)?;

                ctx.ext_mut()
                    .reservation_reply(reservation_id.into(), ReplyPacket::new(payload, value))
                    .map_err(Into::into)
            },
        )
    }

    pub fn reservation_reply_commit(rid_value: ReadAs<HashWithValue>) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReservationReplyCommit,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let HashWithValue {
                    hash: reservation_id,
                    value,
                } = rid_value.read(ctx, io.io_mut_ref()?)?;

                ctx.ext_mut()
                    .reservation_reply_commit(
                        reservation_id.into(),
                        ReplyPacket::new(Default::default(), value),
                    )
                    .map_err(Into::into)
            },
        )
    }

    pub fn reply_to() -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReplyTo,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut().reply_to().map_err(Into::into)
            },
        )
    }

    pub fn signal_from() -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::SignalFrom,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut().signal_from().map_err(Into::into)
            },
        )
    }

    pub fn reply_push(payload: Read) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::ReplyPush(payload.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let payload = payload.read(ctx, io.io_mut_ref()?)?;

                ctx.ext_mut().reply_push(&payload).map_err(Into::into)
            },
        )
    }

    fn reply_input_inner(
        ctx: &mut CallerWrap<Caller>,
        io: &mut MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        offset: u32,
        len: u32,
        gas_limit: Option<u64>,
        value: ReadDecodedSpecial<u128>,
    ) -> Result<MessageId, RunFallibleError> {
        let value = value.read(ctx, io)?;

        // Charge for `len` is inside `reply_push_input`
        ctx.ext_mut().reply_push_input(offset, len)?;

        ctx.ext_mut()
            .reply_commit(ReplyPacket::maybe_with_gas(
                Default::default(),
                gas_limit,
                value,
            ))
            .map_err(Into::into)
    }

    pub fn reply_input(
        offset: u32,
        len: u32,
        value: ReadDecodedSpecial<u128>,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReplyInput,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::reply_input_inner(ctx, io.io_mut_ref()?, offset, len, None, value)
            },
        )
    }

    pub fn reply_input_wgas(
        offset: u32,
        len: u32,
        gas_limit: u64,
        value: ReadDecodedSpecial<u128>,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReplyInputWGas,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::reply_input_inner(ctx, io.io_mut_ref()?, offset, len, Some(gas_limit), value)
            },
        )
    }

    pub fn reply_push_input(offset: u32, len: u32) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::ReplyPushInput,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut()
                    .reply_push_input(offset, len)
                    .map_err(Into::into)
            },
        )
    }

    fn send_input_inner(
        ctx: &mut CallerWrap<Caller>,
        io: &mut MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        pid_value: ReadAs<HashWithValue>,
        offset: u32,
        len: u32,
        gas_limit: Option<u64>,
        delay: u32,
    ) -> Result<MessageId, RunFallibleError> {
        let HashWithValue {
            hash: destination,
            value,
        } = pid_value.read(ctx, io)?;

        let handle = ctx.ext_mut().send_init()?;
        // Charge for `len` inside `send_push_input`
        ctx.ext_mut().send_push_input(handle, offset, len)?;

        ctx.ext_mut()
            .send_commit(
                handle,
                HandlePacket::maybe_with_gas(
                    destination.into(),
                    Default::default(),
                    gas_limit,
                    value,
                ),
                delay,
            )
            .map_err(Into::into)
    }

    pub fn send_input(
        pid_value: ReadAs<HashWithValue>,
        offset: u32,
        len: u32,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::SendInput,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::send_input_inner(ctx, io.io_mut_ref()?, pid_value, offset, len, None, delay)
            },
        )
    }

    pub fn send_input_wgas(
        pid_value: ReadAs<HashWithValue>,
        offset: u32,
        len: u32,
        gas_limit: u64,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::SendInputWGas,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::send_input_inner(
                    ctx,
                    io.io_mut_ref()?,
                    pid_value,
                    offset,
                    len,
                    Some(gas_limit),
                    delay,
                )
            },
        )
    }

    pub fn send_push_input(handle: u32, offset: u32, len: u32) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::SendPushInput,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut()
                    .send_push_input(handle, offset, len)
                    .map_err(Into::into)
            },
        )
    }

    pub fn debug(data: Read) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Debug(data.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let data: RuntimeBuffer = data
                    .read(ctx, io.io_mut_ref()?)?
                    .try_into()
                    .map_err(|RuntimeBufferSizeError| {
                        UnrecoverableMemoryError::RuntimeAllocOutOfBounds.into()
                    })
                    .map_err(TrapExplanation::UnrecoverableExt)?;

                let s = String::from_utf8(data.into_vec())
                    .map_err(|_err| UnrecoverableExecutionError::InvalidDebugString.into())
                    .map_err(TrapExplanation::UnrecoverableExt)?;
                ctx.ext_mut().debug(&s)?;

                Ok(())
            },
        )
    }

    pub fn panic(data: Read) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Null,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let data = data.read(ctx, io.io_mut_ref()?).unwrap_or_default();

                let s = String::from_utf8_lossy(&data).to_string();

                Err(ActorTerminationReason::Trap(TrapExplanation::Panic(s.into())).into())
            },
        )
    }

    pub fn oom_panic() -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Null,
            |_ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                Err(ActorTerminationReason::Trap(TrapExplanation::ProgramAllocOutOfBounds).into())
            },
        )
    }

    pub fn reserve_gas(gas_value: u64, duration: u32) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithHash>(
            CostToken::ReserveGas,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut()
                    .reserve_gas(gas_value, duration)
                    .map_err(Into::into)
            },
        )
    }

    pub fn reply_deposit(
        message_id: ReadDecoded<MessageId>,
        gas_value: u64,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::ReplyDeposit,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let message_id = message_id.read(ctx, io.io_mut_ref()?)?;

                ctx.ext_mut()
                    .reply_deposit(message_id, gas_value)
                    .map_err(Into::into)
            },
        )
    }

    pub fn unreserve_gas(reservation_id: ReadDecoded<ReservationId>) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithGas>(
            CostToken::UnreserveGas,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let reservation_id = reservation_id.read(ctx, io.io_mut_ref()?)?;

                ctx.ext_mut()
                    .unreserve_gas(reservation_id)
                    .map_err(Into::into)
            },
        )
    }

    pub fn system_reserve_gas(gas_value: u64) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::SystemReserveGas,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut()
                    .system_reserve_gas(gas_value)
                    .map_err(Into::into)
            },
        )
    }

    pub fn gas_available(gas: WriteAs<[u8; 8]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::GasAvailable,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let gas_available = ctx.ext_mut().gas_available()?;

                gas.write(ctx, io.io_mut_ref()?, gas_available.to_le_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn message_id(message_id_write: WriteAs<[u8; 32]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::MsgId,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let message_id = ctx.ext_mut().message_id()?;

                message_id_write
                    .write(ctx, io.io_mut_ref()?, message_id.into_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn program_id(program_id_write: WriteAs<[u8; 32]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::ProgramId,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let program_id = ctx.ext_mut().program_id()?;

                program_id_write
                    .write(ctx, io.io_mut_ref()?, program_id.into_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn source(source_write: WriteAs<[u8; 32]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Source,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let source = ctx.ext_mut().source()?;

                source_write
                    .write(ctx, io.io_mut_ref()?, source.into_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn value(value_write: WriteAs<[u8; 16]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Value,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let value = ctx.ext_mut().value()?;

                value_write
                    .write(ctx, io.io_mut_ref()?, value.to_le_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn value_available(value_write: WriteAs<[u8; 16]>) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::ValueAvailable,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let value_available = ctx.ext_mut().value_available()?;

                value_write
                    .write(ctx, io.io_mut_ref()?, value_available.to_le_bytes())
                    .map_err(Into::into)
            },
        )
    }

    pub fn leave() -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Leave,
            move |_ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                Err(ActorTerminationReason::Leave.into())
            },
        )
    }

    pub fn wait() -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Wait,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut().wait()?;
                Err(ActorTerminationReason::Wait(None, MessageWaitedType::Wait).into())
            },
        )
    }

    pub fn wait_for(duration: u32) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::WaitFor,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                ctx.ext_mut().wait_for(duration)?;
                Err(ActorTerminationReason::Wait(Some(duration), MessageWaitedType::WaitFor).into())
            },
        )
    }

    pub fn wait_up_to(duration: u32) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::WaitUpTo,
            move |ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                let waited_type = if ctx.ext_mut().wait_up_to(duration)? {
                    MessageWaitedType::WaitUpToFull
                } else {
                    MessageWaitedType::WaitUpTo
                };
                Err(ActorTerminationReason::Wait(Some(duration), waited_type).into())
            },
        )
    }

    pub fn wake(message_id: ReadDecoded<MessageId>, delay: u32) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorBytes>(
            CostToken::Wake,
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                let message_id = message_id.read(ctx, io.io_mut_ref()?)?;

                ctx.ext_mut().wake(message_id, delay).map_err(Into::into)
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn create_program_inner(
        ctx: &mut CallerWrap<Caller>,
        io: &mut MemoryAccessIo<Caller, BackendMemory<ExecutorMemory>>,
        cid_value: ReadAs<HashWithValue>,
        salt: Read,
        payload: Read,
        gas_limit: Option<u64>,
        delay: u32,
    ) -> Result<(MessageId, ProgramId), RunFallibleError> {
        let HashWithValue {
            hash: code_id,
            value,
        } = cid_value.read(ctx, io)?;
        let salt = Self::read_message_payload(ctx, io, salt)?;
        let payload = Self::read_message_payload(ctx, io, payload)?;

        let message_id = ctx.ext_mut().message_id()?;

        ctx.ext_mut()
            .create_program(
                InitPacket::new_from_program(
                    code_id.into(),
                    salt,
                    payload,
                    message_id,
                    gas_limit,
                    value,
                ),
                delay,
            )
            .map_err(Into::into)
    }

    pub fn create_program(
        cid_value: ReadAs<HashWithValue>,
        salt: Read,
        payload: Read,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithTwoHashes>(
            CostToken::CreateProgram(payload.size().into(), salt.size().into()),
            move |ctx: &mut CallerWrap<Caller>,
                  io: &mut MemoryAccessIoWrapper<Caller>|
                  -> Result<_, RunFallibleError> {
                Self::create_program_inner(
                    ctx,
                    io.io_mut_ref()?,
                    cid_value,
                    salt,
                    payload,
                    None,
                    delay,
                )
            },
        )
    }

    pub fn create_program_wgas(
        cid_value: ReadAs<HashWithValue>,
        salt: Read,
        payload: Read,
        gas_limit: u64,
        delay: u32,
    ) -> impl Syscall<Caller> {
        FallibleSyscall::new::<ErrorWithTwoHashes>(
            CostToken::CreateProgramWGas(payload.size().into(), salt.size().into()),
            move |ctx: &mut CallerWrap<Caller>, io: &mut MemoryAccessIoWrapper<Caller>| {
                Self::create_program_inner(
                    ctx,
                    io.io_mut_ref()?,
                    cid_value,
                    salt,
                    payload,
                    Some(gas_limit),
                    delay,
                )
            },
        )
    }

    pub fn forbidden(_args: &[Value]) -> impl Syscall<Caller> {
        InfallibleSyscall::new(
            CostToken::Null,
            |_ctx: &mut CallerWrap<Caller>, _io: &mut MemoryAccessIoWrapper<Caller>| {
                Err(ActorTerminationReason::Trap(TrapExplanation::ForbiddenFunction).into())
            },
        )
    }

    fn out_of_gas(ctx: &mut CallerWrap<Caller>) -> UndefinedTerminationReason {
        let ext = ctx.ext_mut();
        let current_counter = ext.current_counter_type();
        log::trace!(target: "syscalls", "system_break(OutOfGas): Current counter in global represents {current_counter:?}");

        if current_counter == CounterType::GasAllowance {
            // We manually decrease it to 0 because global won't be affected
            // since it didn't pass comparison to argument of `gas_charge()`
            ext.decrease_current_counter_to(0);
        }

        ActorTerminationReason::from(current_counter).into()
    }

    fn stack_limit_exceeded() -> UndefinedTerminationReason {
        TrapExplanation::StackLimitExceeded.into()
    }

    pub fn system_break(_gas: Gas, code: u32) -> impl Syscall<Caller> {
        RawSyscall::new(move |ctx: &mut CallerWrap<Caller>| {
            // At the instrumentation level, we can only use variants of the `SystemBreakCode`,
            // so we should never reach `unreachable!("{err_msg}")`.
            let termination_reason = SystemBreakCode::try_from(code)
                .map(|system_break_code| match system_break_code {
                    SystemBreakCode::OutOfGas => Self::out_of_gas(ctx),
                    SystemBreakCode::StackLimitExceeded => Self::stack_limit_exceeded(),
                })
                .unwrap_or_else(|e| {
                    let err_msg = format!(
                        "system_break: Invalid system break code. \
                        System break code - {code}. \
                        Got error - {e}"
                    );

                    log::error!("{err_msg}");
                    unreachable!("{err_msg}")
                });
            ctx.set_termination_reason(termination_reason);
            Err(HostError)
        })
    }
}
