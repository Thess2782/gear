// This file is part of Gear.

// Copyright (C) 2021-2024 Gear Technologies Inc.
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

//! Common structures for processing.

use crate::{context::SystemReservationContext, precharge::PreChargeGasOperation};
use actor_system_error::actor_system_error;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
use gear_core::{
    code::{CodeError, InstrumentedCode},
    gas::{GasAllowanceCounter, GasAmount, GasCounter},
    ids::{CodeId, MessageId, ProgramId, ReservationId},
    memory::{MemoryError, MemorySetupError, PageBuf},
    message::{
        ContextStore, Dispatch, DispatchKind, IncomingDispatch, MessageWaitedType, StoredDispatch,
    },
    pages::{numerated::tree::IntervalsTree, GearPage, WasmPage, WasmPagesAmount},
    program::{ActiveProgram, MemoryInfix, ProgramState},
    reservation::{GasReservationMap, GasReserver},
};
pub use gear_core_backend::error::TrapExplanation;
use gear_core_backend::{env::SystemEnvironmentError, error::SystemTerminationReason};
use gear_core_errors::{SignalCode, SimpleExecutionError};

/// Kind of the dispatch result.
#[derive(Clone)]
pub enum DispatchResultKind {
    /// Successful dispatch
    Success,
    /// Trap dispatch.
    Trap(TrapExplanation),
    /// Wait dispatch.
    Wait(Option<u32>, MessageWaitedType),
    /// Exit dispatch.
    Exit(ProgramId),
    /// Gas allowance exceed.
    GasAllowanceExceed,
}

/// Result of the specific dispatch.
pub struct DispatchResult {
    /// Kind of the dispatch.
    pub kind: DispatchResultKind,
    /// Original dispatch.
    pub dispatch: IncomingDispatch,
    /// Program id of actor which was executed.
    pub program_id: ProgramId,
    /// Context store after execution.
    pub context_store: ContextStore,
    /// List of generated messages.
    pub generated_dispatches: Vec<(Dispatch, u32, Option<ReservationId>)>,
    /// List of messages that should be woken.
    pub awakening: Vec<(MessageId, u32)>,
    /// List of reply deposits to be provided.
    pub reply_deposits: Vec<(MessageId, u64)>,
    /// New programs to be created with additional data (corresponding code hash and init message id).
    pub program_candidates: BTreeMap<CodeId, Vec<(MessageId, ProgramId)>>,
    /// Gas amount after execution.
    pub gas_amount: GasAmount,
    /// Gas amount programs reserved.
    pub gas_reserver: Option<GasReserver>,
    /// System reservation context.
    pub system_reservation_context: SystemReservationContext,
    /// Page updates.
    pub page_update: BTreeMap<GearPage, PageBuf>,
    /// New allocations set for program if it has been changed.
    pub allocations: Option<IntervalsTree<WasmPage>>,
    /// Whether this execution sent out a reply.
    pub reply_sent: bool,
}

impl DispatchResult {
    /// Return dispatch message id.
    pub fn message_id(&self) -> MessageId {
        self.dispatch.id()
    }

    /// Return program id.
    pub fn program_id(&self) -> ProgramId {
        self.program_id
    }

    /// Return dispatch source program id.
    pub fn message_source(&self) -> ProgramId {
        self.dispatch.source()
    }

    /// Return dispatch message value.
    pub fn message_value(&self) -> u128 {
        self.dispatch.value()
    }

    /// Create partially initialized instance with the kind
    /// representing Success.
    pub fn success(
        dispatch: IncomingDispatch,
        program_id: ProgramId,
        gas_amount: GasAmount,
    ) -> Self {
        let system_reservation_context = SystemReservationContext::from_dispatch(&dispatch);

        Self {
            kind: DispatchResultKind::Success,
            dispatch,
            program_id,
            context_store: Default::default(),
            generated_dispatches: Default::default(),
            awakening: Default::default(),
            reply_deposits: Default::default(),
            program_candidates: Default::default(),
            gas_amount,
            gas_reserver: None,
            system_reservation_context,
            page_update: Default::default(),
            allocations: Default::default(),
            // This function is only used to generate a dispatch result if nothing is executed,
            // therefore reply_sent will always be false
            reply_sent: false,
        }
    }
}

/// Dispatch outcome of the specific message.
#[derive(Clone, Debug)]
pub enum DispatchOutcome {
    /// Message was a exit.
    Exit {
        /// Id of the program that was successfully exited.
        program_id: ProgramId,
    },
    /// Message was an initialization success.
    InitSuccess {
        /// Id of the program that was successfully initialized.
        program_id: ProgramId,
    },
    /// Message was an initialization failure.
    InitFailure {
        /// Program that was failed initializing.
        program_id: ProgramId,
        /// Source of the init message. Funds inheritor.
        origin: ProgramId,
        /// Reason of the fail.
        reason: String,
    },
    /// Message was a trap.
    MessageTrap {
        /// Program that was failed.
        program_id: ProgramId,
        /// Reason of the fail.
        trap: String,
    },
    /// Message was a success.
    Success,
    /// Message was processed, but not executed
    NoExecution,
}

/// Journal record for the state update.
#[derive(Clone, Debug)]
pub enum JournalNote {
    /// Message was successfully dispatched.
    MessageDispatched {
        /// Message id of dispatched message.
        message_id: MessageId,
        /// Source of the dispatched message.
        source: ProgramId,
        /// Outcome of the processing.
        outcome: DispatchOutcome,
    },
    /// Some gas was burned.
    GasBurned {
        /// Message id in which gas was burned.
        message_id: MessageId,
        /// Amount of gas burned.
        amount: u64,
    },
    /// Exit the program.
    ExitDispatch {
        /// Id of the program called `exit`.
        id_exited: ProgramId,
        /// Address where all remaining value of the program should
        /// be transferred to.
        value_destination: ProgramId,
    },
    /// Message was handled and no longer exists.
    ///
    /// This should be the last update involving this message id.
    MessageConsumed(MessageId),
    /// Message was generated.
    SendDispatch {
        /// Message id of the message that generated this message.
        message_id: MessageId,
        /// New message with entry point that was generated.
        dispatch: Dispatch,
        /// Amount of blocks to wait before sending.
        delay: u32,
        /// Whether use supply from reservation or current message.
        reservation: Option<ReservationId>,
    },
    /// Put this dispatch in the wait list.
    WaitDispatch {
        /// Stored dispatch to be inserted into Waitlist.
        dispatch: StoredDispatch,
        /// Expected duration of holding.
        duration: Option<u32>,
        /// If this message is waiting for its reincarnation.
        waited_type: MessageWaitedType,
    },
    /// Wake particular message.
    WakeMessage {
        /// Message which has initiated wake.
        message_id: MessageId,
        /// Program which has initiated wake.
        program_id: ProgramId,
        /// Message that should be woken.
        awakening_id: MessageId,
        /// Amount of blocks to wait before waking.
        delay: u32,
    },
    /// Update page.
    UpdatePage {
        /// Program that owns the page.
        program_id: ProgramId,
        /// Number of the page.
        page_number: GearPage,
        /// New data of the page.
        data: PageBuf,
    },
    /// Update allocations set note.
    /// And also removes data for pages which is not in allocations set now.
    UpdateAllocations {
        /// Program id.
        program_id: ProgramId,
        /// New allocations set for the program.
        allocations: IntervalsTree<WasmPage>,
    },
    /// Send value
    SendValue {
        /// Value sender
        from: ProgramId,
        /// Value beneficiary,
        to: Option<ProgramId>,
        /// Value amount
        value: u128,
    },
    /// Store programs requested by user to be initialized later
    StoreNewPrograms {
        /// Code hash used to create new programs with ids in `candidates` field
        code_id: CodeId,
        /// Collection of program candidate ids and their init message ids.
        candidates: Vec<(MessageId, ProgramId)>,
    },
    /// Stop processing queue.
    StopProcessing {
        /// Pushes StoredDispatch back to the top of the queue.
        dispatch: StoredDispatch,
        /// Decreases gas allowance by that amount, burned for processing try.
        gas_burned: u64,
    },
    /// Reserve gas.
    ReserveGas {
        /// Message from which gas is reserved.
        message_id: MessageId,
        /// Reservation ID
        reservation_id: ReservationId,
        /// Program which contains reservation.
        program_id: ProgramId,
        /// Amount of reserved gas.
        amount: u64,
        /// How many blocks reservation will live.
        duration: u32,
    },
    /// Unreserve gas.
    UnreserveGas {
        /// Reservation ID
        reservation_id: ReservationId,
        /// Program which contains reservation.
        program_id: ProgramId,
        /// Block number until reservation will live.
        expiration: u32,
    },
    /// Update gas reservation map in program.
    UpdateGasReservations {
        /// Program whose map will be updated.
        program_id: ProgramId,
        /// Map with reservations.
        reserver: GasReserver,
    },
    /// Do system reservation.
    SystemReserveGas {
        /// Message ID which system reservation will be made from.
        message_id: MessageId,
        /// Amount of reserved gas.
        amount: u64,
    },
    /// Do system unreservation in case it is created but not used.
    SystemUnreserveGas {
        /// Message ID which system reservation was made from.
        message_id: MessageId,
    },
    /// Send signal.
    SendSignal {
        /// Message ID which system reservation was made from.
        message_id: MessageId,
        /// Program ID which signal will be sent to.
        destination: ProgramId,
        /// Simple signal error.
        code: SignalCode,
    },
    /// Create deposit for future reply.
    ReplyDeposit {
        /// Message id of the message that generated this message.
        message_id: MessageId,
        /// Future reply id to be sponsored.
        future_reply_id: MessageId,
        /// Amount of gas for reply.
        amount: u64,
    },
}

/// Journal handler.
///
/// Something that can update state.
pub trait JournalHandler {
    /// Process message dispatch.
    fn message_dispatched(
        &mut self,
        message_id: MessageId,
        source: ProgramId,
        outcome: DispatchOutcome,
    );
    /// Process gas burned.
    fn gas_burned(&mut self, message_id: MessageId, amount: u64);
    /// Process exit dispatch.
    fn exit_dispatch(&mut self, id_exited: ProgramId, value_destination: ProgramId);
    /// Process message consumed.
    fn message_consumed(&mut self, message_id: MessageId);
    /// Process send dispatch.
    fn send_dispatch(
        &mut self,
        message_id: MessageId,
        dispatch: Dispatch,
        delay: u32,
        reservation: Option<ReservationId>,
    );
    /// Process send message.
    fn wait_dispatch(
        &mut self,
        dispatch: StoredDispatch,
        duration: Option<u32>,
        waited_type: MessageWaitedType,
    );
    /// Process send message.
    fn wake_message(
        &mut self,
        message_id: MessageId,
        program_id: ProgramId,
        awakening_id: MessageId,
        delay: u32,
    );
    /// Process page update.
    fn update_pages_data(&mut self, program_id: ProgramId, pages_data: BTreeMap<GearPage, PageBuf>);
    /// Process [JournalNote::UpdateAllocations].
    fn update_allocations(&mut self, program_id: ProgramId, allocations: IntervalsTree<WasmPage>);
    /// Send value.
    fn send_value(&mut self, from: ProgramId, to: Option<ProgramId>, value: u128);
    /// Store new programs in storage.
    ///
    /// Program ids are ids of _potential_ (planned to be initialized) programs.
    fn store_new_programs(&mut self, code_id: CodeId, candidates: Vec<(MessageId, ProgramId)>);
    /// Stop processing queue.
    ///
    /// Pushes StoredDispatch back to the top of the queue and decreases gas allowance.
    fn stop_processing(&mut self, dispatch: StoredDispatch, gas_burned: u64);
    /// Reserve gas.
    fn reserve_gas(
        &mut self,
        message_id: MessageId,
        reservation_id: ReservationId,
        program_id: ProgramId,
        amount: u64,
        bn: u32,
    );
    /// Unreserve gas.
    fn unreserve_gas(
        &mut self,
        reservation_id: ReservationId,
        program_id: ProgramId,
        expiration: u32,
    );
    /// Update gas reservations.
    fn update_gas_reservation(&mut self, program_id: ProgramId, reserver: GasReserver);
    /// Do system reservation.
    fn system_reserve_gas(&mut self, message_id: MessageId, amount: u64);
    /// Do system unreservation.
    fn system_unreserve_gas(&mut self, message_id: MessageId);
    /// Send system signal.
    fn send_signal(&mut self, message_id: MessageId, destination: ProgramId, code: SignalCode);
    /// Create deposit for future reply.
    fn reply_deposit(&mut self, message_id: MessageId, future_reply_id: MessageId, amount: u64);
}

actor_system_error! {
    /// Execution error.
    pub type ExecutionError = ActorSystemError<ActorExecutionError, SystemExecutionError>;
}

/// Actor execution error.
#[derive(Debug, derive_more::Display)]
#[display(fmt = "{reason}")]
pub struct ActorExecutionError {
    /// Gas amount of the execution.
    pub gas_amount: GasAmount,
    /// Error text.
    pub reason: ActorExecutionErrorReplyReason,
}

/// Reason of execution error
#[derive(Debug, PartialEq, Eq, derive_more::Display)]
pub enum ActorExecutionErrorReplyReason {
    /// Not enough gas to perform an operation during precharge.
    #[display(fmt = "Not enough gas to {_0}")]
    PreChargeGasLimitExceeded(PreChargeGasOperation),
    /// Backend error
    #[display(fmt = "Environment error: <host error stripped>")]
    Environment,
    /// Trap explanation
    #[display(fmt = "{_0}")]
    Trap(TrapExplanation),
    // TODO: move this to SystemExecutionError after runtime upgrade,
    // if wait-list does not contain messages with total outgoing bytes more than `OutgoingBytesLimit` #3751.
    /// Message is not supported now
    #[display(
        fmt = "Message is not supported: outgoing bytes limit is exceeded after runtime-upgrade"
    )]
    UnsupportedMessage,
}

impl ActorExecutionErrorReplyReason {
    /// Convert self into [`gear_core_errors::SimpleExecutionError`].
    pub fn as_simple(&self) -> SimpleExecutionError {
        match self {
            Self::PreChargeGasLimitExceeded(_) => SimpleExecutionError::RanOutOfGas,
            Self::Trap(expl) => match expl {
                TrapExplanation::GasLimitExceeded => SimpleExecutionError::RanOutOfGas,
                TrapExplanation::ForbiddenFunction | TrapExplanation::UnrecoverableExt(_) => {
                    SimpleExecutionError::BackendError
                }
                TrapExplanation::ProgramAllocOutOfBounds => SimpleExecutionError::MemoryOverflow,
                TrapExplanation::Panic(_) => SimpleExecutionError::UserspacePanic,
                TrapExplanation::StackLimitExceeded => SimpleExecutionError::StackLimitExceeded,
                TrapExplanation::Unknown => SimpleExecutionError::UnreachableInstruction,
            },
            Self::Environment | Self::UnsupportedMessage => SimpleExecutionError::Unsupported,
        }
    }
}

/// System execution error
#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum SystemExecutionError {
    /// Incorrect memory parameters
    #[from]
    #[display(fmt = "Memory parameters error: {_0}")]
    MemoryParams(MemorySetupError),
    /// Environment error
    #[display(fmt = "Backend error: {_0}")]
    Environment(SystemEnvironmentError),
    /// Termination reason
    #[from]
    #[display(fmt = "Syscall function error: {_0}")]
    UndefinedTerminationReason(SystemTerminationReason),
    /// Error during `into_ext_info()` call
    #[display(fmt = "`into_ext_info()` error: {_0}")]
    IntoExtInfo(MemoryError),
    // TODO: uncomment when #3751
    // /// Incoming dispatch store has too many outgoing messages total bytes.
    // #[display(fmt = "Incoming dispatch store has too many outgoing messages total bytes")]
    // MessageStoreOutgoingBytesOverflow,
}

/// Executable actor data.
#[derive(Clone, Debug)]
pub struct ExecutableActorData {
    /// Set of wasm pages, which are allocated by the program.
    pub allocations: IntervalsTree<WasmPage>,
    /// The infix of memory pages in a storage.
    pub memory_infix: MemoryInfix,
    /// Id of the program code.
    pub code_id: CodeId,
    /// Exported functions by the program code.
    pub code_exports: BTreeSet<DispatchKind>,
    /// Count of static memory pages.
    pub static_pages: WasmPagesAmount,
    /// Gas reservation map.
    pub gas_reservation_map: GasReservationMap,
}

/// Program.
#[derive(Clone, Debug)]
pub(crate) struct Program {
    /// Program id.
    pub id: ProgramId,
    /// Memory infix.
    pub memory_infix: MemoryInfix,
    /// Instrumented code.
    pub code: InstrumentedCode,
    /// Allocations.
    pub allocations: IntervalsTree<WasmPage>,
}

/// Execution context.
#[derive(Debug)]
pub(crate) struct WasmExecutionContext {
    /// A counter for gas.
    pub gas_counter: GasCounter,
    /// A counter for gas allowance.
    pub gas_allowance_counter: GasAllowanceCounter,
    /// Gas reserver.
    pub gas_reserver: GasReserver,
    /// Program to be executed.
    pub program: Program,
    /// Size of the memory block.
    pub memory_size: WasmPagesAmount,
}

/// +_+_+
pub struct ProgramInfo {
    /// +_+_+
    pub allocations: IntervalsTree<WasmPage>,
    /// +_+_+
    pub code_id: CodeId,
    /// +_+_+
    pub code_exports: BTreeSet<DispatchKind>,
    /// +_+_+
    pub memory_infix: MemoryInfix,
    /// +_+_+
    pub gas_reservation_map: GasReservationMap,
    /// +_+_+
    pub state: ProgramState,
}

impl<BlockNumber: Copy> From<ActiveProgram<BlockNumber>> for ProgramInfo {
    fn from(program: ActiveProgram<BlockNumber>) -> Self {
        Self {
            allocations: program.allocations,
            code_id: CodeId::from(program.code_hash.as_bytes()),
            code_exports: program.code_exports,
            memory_infix: program.memory_infix,
            gas_reservation_map: program.gas_reservation_map,
            state: program.state,
        }
    }
}

/// +_+_+
pub trait LazyStorageAccess {
    /// Get program info.
    fn program_info(&self, program_id: ProgramId) -> Option<ProgramInfo>;

    /// Get code length.
    fn code_len(&self, code_id: CodeId) -> Option<u32>;

    /// Get code info.
    fn code(&self, code_id: CodeId) -> Option<InstrumentedCode>;

    /// +_+_+
    fn need_reinstrumentation(&self, code: &InstrumentedCode) -> bool;

    /// +_+_+
    fn reinstrument_code(&self, code_id: CodeId) -> Result<InstrumentedCode, CodeError>;
}
