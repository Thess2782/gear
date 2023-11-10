// This file is part of Gear.

// Copyright (C) 2021-2023 Gear Technologies Inc.
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

//! sp-sandbox extensions for memory.

use crate::{
    error::{
        BackendSyscallError, RunFallibleError, TrapExplanation, UndefinedTerminationReason,
        UnrecoverableMemoryError,
    },
    state::HostState,
    BackendExternalities,
};
use alloc::vec::Vec;
use codec::{Decode, DecodeAll, MaxEncodedLen};
use core::{marker::PhantomData, mem, mem::MaybeUninit, slice};
use gear_core::{
    buffer::{RuntimeBuffer, RuntimeBufferSizeError},
    env::Externalities,
    memory::{HostPointer, Memory, MemoryError, MemoryInterval},
    pages::WasmPagesAmount,
};
use gear_core_errors::MemoryError as FallibleMemoryError;
use gear_lazy_pages_common::ProcessAccessError;
use gear_sandbox::{
    default_executor::{Caller, Store},
    SandboxMemory,
};

pub type ExecutorMemory = gear_sandbox::default_executor::Memory;

pub(crate) struct MemoryWrapRef<'a, 'b: 'a, Ext: Externalities + 'static> {
    pub memory: ExecutorMemory,
    pub caller: &'a mut Caller<'b, HostState<Ext, ExecutorMemory>>,
}

impl<Ext: Externalities + 'static> Memory for MemoryWrapRef<'_, '_, Ext> {
    fn size(&self) -> WasmPagesAmount {
        WasmPagesAmount::try_from(self.memory.size(self.caller))
            .expect("Unexpected executor behavior: wasm size is bigger then 4 GB")
    }

    fn write(&mut self, offset: u32, buffer: &[u8]) -> Result<(), MemoryError> {
        self.memory
            .write(self.caller, offset, buffer)
            .map_err(|_| MemoryError::AccessOutOfBounds)
    }

    fn read(&self, offset: u32, buffer: &mut [u8]) -> Result<(), MemoryError> {
        self.memory
            .read(self.caller, offset, buffer)
            .map_err(|_| MemoryError::AccessOutOfBounds)
    }

    unsafe fn get_buffer_host_addr_unsafe(&mut self) -> HostPointer {
        self.memory.get_buff(self.caller) as HostPointer
    }
}

/// Wrapper for executor memory.
pub struct MemoryWrap<Ext>
where
    Ext: Externalities + 'static,
{
    pub(crate) memory: ExecutorMemory,
    pub(crate) store: Store<HostState<Ext, ExecutorMemory>>,
}

impl<Ext> MemoryWrap<Ext>
where
    Ext: Externalities + 'static,
{
    /// Wrap [`ExecutorMemory`] for Memory trait.
    pub fn new(memory: ExecutorMemory, store: Store<HostState<Ext, ExecutorMemory>>) -> Self {
        MemoryWrap { memory, store }
    }

    pub(crate) fn into_store(self) -> Store<HostState<Ext, ExecutorMemory>> {
        self.store
    }
}

/// Memory interface for the allocator.
impl<Ext> Memory for MemoryWrap<Ext>
where
    Ext: Externalities + 'static,
{
    fn size(&self) -> WasmPagesAmount {
        self.memory
            .size(&self.store)
            .try_into()
            .expect("Unexpected backend behavior: wasm size is bigger than 4 GB")
    }

    fn write(&mut self, offset: u32, buffer: &[u8]) -> Result<(), MemoryError> {
        self.memory
            .write(&mut self.store, offset, buffer)
            .map_err(|_| MemoryError::AccessOutOfBounds)
    }

    fn read(&self, offset: u32, buffer: &mut [u8]) -> Result<(), MemoryError> {
        self.memory
            .read(&self.store, offset, buffer)
            .map_err(|_| MemoryError::AccessOutOfBounds)
    }

    unsafe fn get_buffer_host_addr_unsafe(&mut self) -> HostPointer {
        self.memory.get_buff(&mut self.store) as HostPointer
    }
}

#[derive(Debug, Clone, derive_more::From)]
pub(crate) enum MemoryAccessError {
    Memory(MemoryError),
    ProcessAccess(ProcessAccessError),
    RuntimeBuffer(RuntimeBufferSizeError),
    // TODO: remove #2164
    Decode,
}

impl BackendSyscallError for MemoryAccessError {
    fn into_termination_reason(self) -> UndefinedTerminationReason {
        match self {
            MemoryAccessError::ProcessAccess(ProcessAccessError::OutOfBounds)
            | MemoryAccessError::Memory(MemoryError::AccessOutOfBounds) => {
                TrapExplanation::UnrecoverableExt(
                    UnrecoverableMemoryError::AccessOutOfBounds.into(),
                )
                .into()
            }
            MemoryAccessError::RuntimeBuffer(RuntimeBufferSizeError) => {
                TrapExplanation::UnrecoverableExt(
                    UnrecoverableMemoryError::RuntimeAllocOutOfBounds.into(),
                )
                .into()
            }
            // TODO: In facts thats legacy from lazy pages V1 implementation,
            // previously it was able to figure out that gas ended up in
            // pre-process charges: now we need actual counter type, so
            // it will be parsed and handled further (issue #3018).
            MemoryAccessError::ProcessAccess(ProcessAccessError::GasLimitExceeded) => {
                UndefinedTerminationReason::ProcessAccessErrorResourcesExceed
            }
            MemoryAccessError::Decode => unreachable!(),
        }
    }

    fn into_run_fallible_error(self) -> RunFallibleError {
        match self {
            MemoryAccessError::Memory(MemoryError::AccessOutOfBounds)
            | MemoryAccessError::ProcessAccess(ProcessAccessError::OutOfBounds) => {
                RunFallibleError::FallibleExt(FallibleMemoryError::AccessOutOfBounds.into())
            }
            MemoryAccessError::RuntimeBuffer(RuntimeBufferSizeError) => {
                RunFallibleError::FallibleExt(FallibleMemoryError::RuntimeAllocOutOfBounds.into())
            }
            e => RunFallibleError::UndefinedTerminationReason(e.into_termination_reason()),
        }
    }
}

/// Memory access manager. Allows to pre-register memory accesses,
/// and pre-process, them together. For example:
/// ```ignore
/// let manager = MemoryAccessManager::default();
/// let read1 = manager.new_read(10, 20);
/// let read2 = manager.new_read_as::<u128>(100);
/// let write1 = manager.new_write_as::<usize>(190);
///
/// // First call of read or write interface leads to pre-processing of
/// // all already registered memory accesses, and clear `self.reads` and `self.writes`.
/// let value_u128 = manager.read_as(read2).unwrap();
///
/// // Next calls do not lead to access pre-processing.
/// let value1 = manager.read().unwrap();
/// manager.write_as(write1, 111).unwrap();
/// ```
#[derive(Debug)]
pub(crate) struct MemoryAccessManager<Ext> {
    // Contains non-zero length intervals only.
    pub(crate) reads: Vec<MemoryInterval>,
    pub(crate) writes: Vec<MemoryInterval>,
    pub(crate) _phantom: PhantomData<Ext>,
}

impl<Ext> Default for MemoryAccessManager<Ext> {
    fn default() -> Self {
        Self {
            reads: Vec::new(),
            writes: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<Ext> MemoryAccessManager<Ext> {
    pub fn register_read(&mut self, ptr: u32, size: u32) -> WasmMemoryRead {
        if size > 0 {
            self.reads.push(MemoryInterval { offset: ptr, size });
        }
        WasmMemoryRead { ptr, size }
    }

    pub fn register_read_as<T: Sized>(&mut self, ptr: u32) -> WasmMemoryReadAs<T> {
        let size = mem::size_of::<T>() as u32;
        if size > 0 {
            self.reads.push(MemoryInterval { offset: ptr, size });
        }
        WasmMemoryReadAs {
            ptr,
            _phantom: PhantomData,
        }
    }

    pub fn register_read_decoded<T: Decode + MaxEncodedLen>(
        &mut self,
        ptr: u32,
    ) -> WasmMemoryReadDecoded<T> {
        let size = T::max_encoded_len() as u32;
        if size > 0 {
            self.reads.push(MemoryInterval { offset: ptr, size });
        }
        WasmMemoryReadDecoded {
            ptr,
            _phantom: PhantomData,
        }
    }

    pub fn register_write(&mut self, ptr: u32, size: u32) -> WasmMemoryWrite {
        if size > 0 {
            self.writes.push(MemoryInterval { offset: ptr, size });
        }
        WasmMemoryWrite { ptr, size }
    }

    pub fn register_write_as<T: Sized>(&mut self, ptr: u32) -> WasmMemoryWriteAs<T> {
        let size = mem::size_of::<T>() as u32;
        if size > 0 {
            self.writes.push(MemoryInterval { offset: ptr, size });
        }
        WasmMemoryWriteAs {
            ptr,
            _phantom: PhantomData,
        }
    }
}

impl<Ext: BackendExternalities> MemoryAccessManager<Ext> {
    /// Call pre-processing of registered memory accesses. Clear `self.reads` and `self.writes`.
    pub(crate) fn pre_process_memory_accesses(
        &mut self,
        gas_counter: &mut u64,
    ) -> Result<(), MemoryAccessError> {
        if self.reads.is_empty() && self.writes.is_empty() {
            return Ok(());
        }

        let res = Ext::pre_process_memory_accesses(&self.reads, &self.writes, gas_counter);

        self.reads.clear();
        self.writes.clear();

        res.map_err(Into::into)
    }

    /// Pre-process registered accesses if need and read data from `memory` to `buff`.
    fn read_into_buf<M: Memory>(
        &mut self,
        memory: &M,
        ptr: u32,
        buff: &mut [u8],
        gas_counter: &mut u64,
    ) -> Result<(), MemoryAccessError> {
        self.pre_process_memory_accesses(gas_counter)?;
        memory.read(ptr, buff).map_err(Into::into)
    }

    /// Pre-process registered accesses if need and read data from `memory` into new vector.
    pub fn read<M: Memory>(
        &mut self,
        memory: &M,
        read: WasmMemoryRead,
        gas_counter: &mut u64,
    ) -> Result<Vec<u8>, MemoryAccessError> {
        let buff = if read.size == 0 {
            Vec::new()
        } else {
            let mut buff = RuntimeBuffer::try_new_default(read.size as usize)?.into_vec();
            self.read_into_buf(memory, read.ptr, &mut buff, gas_counter)?;
            buff
        };
        Ok(buff)
    }

    /// Pre-process registered accesses if need and read and decode data as `T` from `memory`.
    pub fn read_decoded<M: Memory, T: Decode + MaxEncodedLen>(
        &mut self,
        memory: &M,
        read: WasmMemoryReadDecoded<T>,
        gas_counter: &mut u64,
    ) -> Result<T, MemoryAccessError> {
        let size = T::max_encoded_len();
        let buff = if size == 0 {
            Vec::new()
        } else {
            let mut buff = RuntimeBuffer::try_new_default(size)?.into_vec();
            self.read_into_buf(memory, read.ptr, &mut buff, gas_counter)?;
            buff
        };
        let decoded = T::decode_all(&mut &buff[..]).map_err(|_| MemoryAccessError::Decode)?;
        Ok(decoded)
    }

    /// Pre-process registered accesses if need and read data as `T` from `memory`.
    pub fn read_as<M: Memory, T: Sized>(
        &mut self,
        memory: &M,
        read: WasmMemoryReadAs<T>,
        gas_counter: &mut u64,
    ) -> Result<T, MemoryAccessError> {
        self.pre_process_memory_accesses(gas_counter)?;
        read_memory_as(memory, read.ptr).map_err(Into::into)
    }

    /// Pre-process registered accesses if need and write data from `buff` to `memory`.
    pub fn write<M: Memory>(
        &mut self,
        memory: &mut M,
        write: WasmMemoryWrite,
        buff: &[u8],
        gas_counter: &mut u64,
    ) -> Result<(), MemoryAccessError> {
        if buff.len() != write.size as usize {
            unreachable!("Backend bug error: buffer size is not equal to registered buffer size");
        }
        if write.size == 0 {
            Ok(())
        } else {
            self.pre_process_memory_accesses(gas_counter)?;
            memory.write(write.ptr, buff).map_err(Into::into)
        }
    }

    /// Pre-process registered accesses if need and write `obj` data to `memory`.
    pub fn write_as<M: Memory, T: Sized>(
        &mut self,
        memory: &mut M,
        write: WasmMemoryWriteAs<T>,
        obj: T,
        gas_counter: &mut u64,
    ) -> Result<(), MemoryAccessError> {
        self.pre_process_memory_accesses(gas_counter)?;
        write_memory_as(memory, write.ptr, obj).map_err(Into::into)
    }
}

/// Writes object in given memory as bytes.
fn write_memory_as<T: Sized>(
    memory: &mut impl Memory,
    ptr: u32,
    obj: T,
) -> Result<(), MemoryError> {
    let size = mem::size_of::<T>();
    if size > 0 {
        // # Safety:
        //
        // Given object is `Sized` and we own them in the context of calling this
        // function (it's on stack), it's safe to take ptr on the object and
        // represent it as slice. Object will be dropped after `memory.write`
        // finished execution and no one will rely on this slice.
        //
        // Bytes in memory always stored continuously and without paddings, properly
        // aligned due to `[repr(C, packed)]` attribute of the types we use as T.
        let slice = unsafe { slice::from_raw_parts(&obj as *const T as *const u8, size) };

        memory.write(ptr, slice)
    } else {
        Ok(())
    }
}

/// Reads bytes from given pointer to construct type T from them.
fn read_memory_as<T: Sized>(memory: &impl Memory, ptr: u32) -> Result<T, MemoryError> {
    let mut buf = MaybeUninit::<T>::uninit();

    let size = mem::size_of::<T>();
    if size > 0 {
        // # Safety:
        //
        // Usage of mutable slice is safe for the same reason from `write_memory_as`.
        // `MaybeUninit` is presented on stack as a contiguous sequence of bytes.
        //
        // It's also safe to construct T from any bytes, because we use the fn
        // only for reading primitive const-size types that are `[repr(C)]`,
        // so they always represented from sequence of bytes.
        //
        // Bytes in memory always stored continuously and without paddings, properly
        // aligned due to `[repr(C, packed)]` attribute of the types we use as T.
        let mut_slice = unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, size) };

        memory.read(ptr, mut_slice)?;
    }

    // # Safety:
    //
    // Assuming init is always safe here due to the fact that we read proper
    // amount of bytes from the wasm memory, which is never uninited: they may
    // be filled by zeroes or some trash (valid for our primitives used as T),
    // but always exist.
    Ok(unsafe { buf.assume_init() })
}

/// Read static size type access wrapper.
pub(crate) struct WasmMemoryReadAs<T> {
    pub(crate) ptr: u32,
    pub(crate) _phantom: PhantomData<T>,
}

/// Read decoded type access wrapper.
pub(crate) struct WasmMemoryReadDecoded<T: Decode + MaxEncodedLen> {
    pub(crate) ptr: u32,
    pub(crate) _phantom: PhantomData<T>,
}

/// Read access wrapper.
pub(crate) struct WasmMemoryRead {
    pub(crate) ptr: u32,
    pub(crate) size: u32,
}

/// Write static size type access wrapper.
pub(crate) struct WasmMemoryWriteAs<T> {
    pub(crate) ptr: u32,
    pub(crate) _phantom: PhantomData<T>,
}

/// Write access wrapper.
pub(crate) struct WasmMemoryWrite {
    pub(crate) ptr: u32,
    pub(crate) size: u32,
}

// +_+_+ Move to core/src/memory.rs
// // can't be tested outside the node runtime
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{error::ActorTerminationReason, mock::MockExt, state::State};
//     use gear_core::{
//         memory::{AllocError, AllocationsContext},
//         pages::WasmPage,
//     };
//     use gear_sandbox::{AsContextExt, SandboxStore};

//     fn new_test_memory(
//         static_pages: u16,
//         mem_size: u16,
//     ) -> (AllocationsContext, MemoryWrap<MockExt>) {
//         use gear_sandbox::SandboxMemory as WasmMemory;

//         let mut store = Store::new(None);
//         let memory: ExecutorMemory =
//             WasmMemory::new(&mut store, mem_size as u32, None).expect("Memory creation failed");
//         *store.data_mut() = Some(State {
//             ext: MockExt::default(),
//             memory: memory.clone(),
//             termination_reason: ActorTerminationReason::Success.into(),
//         });

//         let memory = MemoryWrap::new(memory, store);

//         (
//             AllocationsContext::new(Default::default(), static_pages.into(), mem_size.into()),
//             memory,
//         )
//     }

//     #[test]
//     fn smoky() {
//         let _ = env_logger::try_init();

//         let (mut ctx, mut mem_wrap) = new_test_memory(16, 256);

//         assert_eq!(ctx.alloc(16.into()), Ok(16.into()));

//         assert_eq!(ctx.alloc(0.into()).unwrap(), Ok(32.into()));

//         // there is a space for 14 more
//         for _ in 0..14 {
//             ctx.alloc::<NoopGrowHandler>(16.into()).unwrap();
//         }

//         // no more mem!
//         assert_eq!(
//             ctx.alloc(1.into()),
//             Err(AllocError::ProgramAllocOutOfBounds)
//         );

//         // but we free some
//         ctx.free(137.into()).unwrap();

//         // and now can allocate page that was freed
//         assert_eq!(
//             ctx.alloc::<NoopGrowHandler>(1.into),
//             Ok(137.into())
//         );

//         // if we have 2 in a row we can allocate even 2
//         ctx.free(117.into()).unwrap();
//         ctx.free(118.into()).unwrap();

//         assert_eq!(
//             ctx.alloc::<NoopGrowHandler>(2.into(), &mut mem_wrap, |_| Ok(())),
//             Ok(117.into())
//         );

//         // but if 2 are not in a row, bad luck
//         ctx.free(117.into()).unwrap();
//         ctx.free(158.into()).unwrap();

//         assert_eq!(
//             ctx.alloc::<NoopGrowHandler>(2.into(), &mut mem_wrap, |_| Ok(())),
//             Err(AllocError::ProgramAllocOutOfBounds)
//         );
//     }
// }
