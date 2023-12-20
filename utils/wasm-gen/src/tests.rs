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

use super::*;
use arbitrary::Unstructured;
use gear_core::{
    code::Code,
    gas::ValueCounter,
    ids::{CodeId, ProgramId},
    memory::Memory,
    message::{
        ContextSettings, DispatchKind, IncomingDispatch, IncomingMessage, MessageContext,
        ReplyPacket,
    },
    pages::WASM_PAGE_SIZE,
};
use gear_core_backend::{
    env::{BackendReport, Environment},
    error::{ActorTerminationReason, TerminationReason, TrapExplanation},
};
use gear_core_processor::{ProcessorContext, ProcessorExternalities};
use gear_utils::NonEmpty;
use gear_wasm_instrument::{
    parity_wasm::{
        self,
        elements::{Instruction, Module},
    },
    rules::CustomConstantCostRules,
};
use proptest::prelude::*;
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use std::{mem, num::NonZeroUsize};

const UNSTRUCTURED_SIZE: usize = 1_000_000;

#[test]
fn inject_critical_gas_limit_works() {
    let wat1 = r#"
    (module
        (memory $memory0 (import "env" "memory") 16)
        (export "handle" (func $handle))
        (func $handle
            call $f
            drop
        )
        (func $f (result i64)
            call $f
        )
        (func $g
            (loop $my_loop
                br $my_loop
            )
        )
    )"#;

    let wasm_bytes = wat::parse_str(wat1).expect("invalid wat");
    let module =
        parity_wasm::deserialize_buffer::<Module>(&wasm_bytes).expect("invalid wasm bytes");
    let module_with_critical_gas_limit = utils::inject_critical_gas_limit(module, 1_000_000);

    let wasm_bytes = module_with_critical_gas_limit
        .into_bytes()
        .expect("invalid pw module");
    assert!(wasmparser::validate(&wasm_bytes).is_ok());

    let wat = wasmprinter::print_bytes(&wasm_bytes).expect("failed printing bytes");
    println!("wat = {wat}");
}

#[test]
fn remove_trivial_recursions() {
    let wat1 = r#"
    (module
        (func (;0;)
            call 0
        )
    )"#;

    let wasm_bytes = wat::parse_str(wat1).expect("invalid wat");
    let module =
        parity_wasm::deserialize_buffer::<Module>(&wasm_bytes).expect("invalid wasm bytes");
    let no_recursions_module = utils::remove_recursion(module);

    let wasm_bytes = no_recursions_module
        .into_bytes()
        .expect("invalid pw module");
    assert!(wasmparser::validate(&wasm_bytes).is_ok());

    let wat = wasmprinter::print_bytes(&wasm_bytes).expect("failed printing bytes");
    println!("wat = {wat}");
}

#[test]
fn remove_multiple_recursions() {
    let wat2 = r#"
    (module
        (func (;0;) (result i64)
            call 1
        )
        (func (;1;) (result i64)
            call 0
        )
        (func (;2;)
            call 1
            drop
        )
    )"#;

    let wasm_bytes = wat::parse_str(wat2).expect("invalid wat");
    let module =
        parity_wasm::deserialize_buffer::<Module>(&wasm_bytes).expect("invalid wasm bytes");
    utils::find_recursion(&module, |path, call| {
        println!("path = {path:?}, call = {call}");
    });
    let no_recursions_module = utils::remove_recursion(module);
    utils::find_recursion(&no_recursions_module, |_, _| {
        unreachable!("there should be no recursions")
    });

    let wasm_bytes = no_recursions_module
        .into_bytes()
        .expect("invalid pw module");
    assert!(wasmparser::validate(&wasm_bytes).is_ok());

    let wat = wasmprinter::print_bytes(&wasm_bytes).expect("failed printing bytes");
    println!("wat = {wat}");
}

#[test]
fn injecting_addresses_works() {
    let mut rng = SmallRng::seed_from_u64(1234);
    let mut buf = vec![0; UNSTRUCTURED_SIZE];
    rng.fill_bytes(&mut buf);
    let mut u = Unstructured::new(&buf);

    let stack_end_page = 16;
    let addresses = NonEmpty::from_vec(vec![[0; 32], [1; 32]]).expect("vec wasn't empty");
    let config = GearWasmGeneratorConfigBuilder::new()
        .with_memory_config(MemoryPagesConfig {
            initial_size: 17,
            upper_limit: None,
            stack_end_page: Some(stack_end_page),
        })
        .with_syscalls_config(
            SyscallsConfigBuilder::new(Default::default())
                .with_addresses_msg_dest(addresses)
                .build(),
        )
        .build();
    let wasm_module = GearWasmGenerator::new_with_config(
        WasmModule::generate(&mut u).expect("failed module generation"),
        &mut u,
        config,
    )
    .generate()
    .expect("failed gear-wasm generation");

    let data_sections_entries_num = wasm_module
        .data_section()
        .expect("additional data was inserted")
        .entries()
        .len();
    // 2 addresses in the upper `addresses`.
    assert_eq!(data_sections_entries_num, 2);

    let size = mem::size_of::<gsys::HashWithValue>() as i32;
    let entries = wasm_module
        .data_section()
        .expect("additional data was inserted")
        .entries();

    let first_addr_offset = entries
        .get(0)
        .and_then(|segment| segment.offset().as_ref())
        .map(|expr| &expr.code()[0])
        .expect("checked");
    let Instruction::I32Const(ptr) = first_addr_offset else {
        panic!("invalid instruction in init expression")
    };
    // No additional data, except for addresses.
    // First entry set to the 0 offset.
    assert_eq!(*ptr, (stack_end_page * WASM_PAGE_SIZE as u32) as i32);

    let second_addr_offset = entries
        .get(1)
        .and_then(|segment| segment.offset().as_ref())
        .map(|expr| &expr.code()[0])
        .expect("checked");
    let Instruction::I32Const(ptr) = second_addr_offset else {
        panic!("invalid instruction in init expression")
    };
    // No additional data, except for addresses.
    // First entry set to the 0 offset.
    assert_eq!(*ptr, size + (stack_end_page * WASM_PAGE_SIZE as u32) as i32);
}

// #[test]
// fn ptr_setters_work() {
//     const INITIAL_VALUE: u128 = 10_000;
//     const REPLY_VALUE: u128 = 1_000;

//     let buf = vec![0; UNSTRUCTURED_SIZE];
//     let mut unstructured = Unstructured::new(&buf);

//     let params_config = SysCallsParamsConfig::default();
//     let mut pointer_writes_config = PtrParamFillersConfig::empty();
//     pointer_writes_config.set_rule(
//         PtrType::Value,
//         vec![PtrParamAllowedValuesExt {
//             value_offset: 0,
//             allowed_values: FillingPtrParamData::U128(REPLY_VALUE..=REPLY_VALUE),
//         }],
//     );

//     let mut injection_types = SysCallsInjectionTypes::all_never();
//     injection_types.set(InvocableSysCall::Loose(SysCallName::Reply), 1, 1);
//     let sys_calls_config = SysCallsConfigBuilder::new(injection_types)
//         .with_params_config(params_config)
//         .with_pointer_writes_config(pointer_writes_config)
//         .with_error_processing_config(ErrorProcessingConfig::All)
//         .build();

//     let backend_report = execute_wasm_with_custom_configs(
//         &mut unstructured,
//         sys_calls_config,
//         None,
//         1,
//         false,
//         INITIAL_VALUE,
//     );

//     assert_eq!(
//         backend_report.ext.context.value_counter.left(),
//         INITIAL_VALUE - REPLY_VALUE
//     );
//     assert_eq!(
//         backend_report.termination_reason,
//         TerminationReason::Actor(ActorTerminationReason::Success)
//     );
// }

#[test]
fn error_processing_works_for_fallible_syscalls() {
    gear_utils::init_default_logger();

    // We create Unstructured from zeroes here as we just need any.
    let buf = vec![0; UNSTRUCTURED_SIZE];
    let mut unstructured = Unstructured::new(&buf);
    let mut unstructured2 = Unstructured::new(&buf);

    let fallible_syscalls = SyscallName::instrumentable()
        .into_iter()
        .filter_map(|syscall| {
            let invocable_syscall = InvocableSyscall::Loose(syscall);
            invocable_syscall.is_fallible().then_some(invocable_syscall)
        });

    for syscall in fallible_syscalls {
        // Prepare syscalls config & context settings for test case.
        let (params_config, initial_memory_write) = get_params_for_syscall_to_fail(syscall);

        const INJECTED_SYSCALLS: u32 = 8;

        let mut injection_types = SyscallsInjectionTypes::all_never();
        injection_types.set(syscall, INJECTED_SYSCALLS, INJECTED_SYSCALLS);

        let syscalls_config_builder =
            SyscallsConfigBuilder::new(injection_types).with_params_config(params_config);

        // Assert that syscalls results will be processed.
        let termination_reason = execute_wasm_with_custom_configs(
            &mut unstructured,
            syscalls_config_builder
                .clone()
                .with_error_processing_config(ErrorProcessingConfig::All)
                .build(),
            initial_memory_write.clone(),
            0,
            true,
            0,
        )
        .termination_reason;

        assert_eq!(
            termination_reason,
            TerminationReason::Actor(ActorTerminationReason::Trap(TrapExplanation::Unknown)),
            "syscall: {}",
            syscall.to_str()
        );

        // Assert that syscall results will be ignored.
        let termination_reason = execute_wasm_with_custom_configs(
            &mut unstructured2,
            syscalls_config_builder.build(),
            initial_memory_write.clone(),
            0,
            true,
            0,
        )
        .termination_reason;

        assert_eq!(
            termination_reason,
            TerminationReason::Actor(ActorTerminationReason::Success),
            "syscall: {}",
            syscall.to_str()
        );
    }
}

#[test]
fn precise_syscalls_works() {
    use gear_core_backend::error::ActorTerminationReason;

    gear_utils::init_default_logger();

    // Pin a specific seed for this test.
    let mut rng = SmallRng::seed_from_u64(1234);
    let mut buf = vec![0; UNSTRUCTURED_SIZE];
    rng.fill_bytes(&mut buf);
    let mut unstructured = Unstructured::new(&buf);

    let precise_syscalls = SyscallName::instrumentable()
        .into_iter()
        .filter_map(|syscall| {
            InvocableSyscall::has_precise_variant(syscall)
                .then_some(InvocableSyscall::Precise(syscall))
        });

    for syscall in precise_syscalls {
        // Prepare syscalls config & context settings for test case.
        const INJECTED_SYSCALLS: u32 = 1;

        let mut injection_types = SyscallsInjectionTypes::all_never();
        injection_types.set(syscall, INJECTED_SYSCALLS, INJECTED_SYSCALLS);

        let mut param_config = SyscallsParamsConfig::default_regular();
        param_config.set_rule(RegularParamType::Gas, (0..=0).into());

        // Assert that syscalls results will be processed.
        let termination_reason = execute_wasm_with_custom_configs(
            &mut unstructured,
            SyscallsConfigBuilder::new(injection_types)
                .with_params_config(param_config)
                .with_precise_syscalls_config(PreciseSyscallsConfig::new(3..=3, 3..=3))
                .with_source_msg_dest()
                .with_error_processing_config(ErrorProcessingConfig::All)
                .build(),
            None,
            1024,
            false,
            0,
        )
        .termination_reason;

        assert_eq!(
            termination_reason,
            TerminationReason::Actor(ActorTerminationReason::Success),
            "syscall: {}",
            syscall.to_str()
        );
    }
}

#[derive(Clone)]
struct MemoryWrite {
    offset: u32,
    content: Vec<u8>,
}

fn get_params_for_syscall_to_fail(
    syscall: InvocableSyscall,
) -> (SyscallsParamsConfig, Option<MemoryWrite>) {
    let syscall_name = match syscall {
        InvocableSyscall::Loose(name) => name,
        InvocableSyscall::Precise(name) => name,
    };
    let memory_write = match syscall_name {
        SyscallName::PayProgramRent => Some(MemoryWrite {
            offset: 0,
            content: vec![255; WASM_PAGE_SIZE],
        }),
        _ => None,
    };

    (
        SyscallsParamsConfig::const_regular_params(i32::MAX as i64),
        memory_write,
    )
}

fn execute_wasm_with_custom_configs(
    unstructured: &mut Unstructured,
    syscalls_config: SyscallsConfig,
    initial_memory_write: Option<MemoryWrite>,
    outgoing_limit: u32,
    imitate_reply: bool,
    value: u128,
) -> BackendReport<gear_core_processor::Ext> {
    const PROGRAM_STORAGE_PREFIX: [u8; 32] = *b"execute_wasm_with_custom_configs";
    const INITIAL_PAGES: u16 = 1;

    assert!(gear_lazy_pages_interface::try_to_enable_lazy_pages(
        PROGRAM_STORAGE_PREFIX
    ));

    let gear_config = (
        GearWasmGeneratorConfigBuilder::new()
            .with_memory_config(MemoryPagesConfig {
                initial_size: INITIAL_PAGES as u32,
                ..MemoryPagesConfig::default()
            })
            .with_syscalls_config(syscalls_config)
            .with_entry_points_config(EntryPointsSet::Init)
            .build(),
        SelectableParams {
            allowed_instructions: vec![],
            max_instructions: 0,
            min_funcs: NonZeroUsize::new(1).unwrap(),
            max_funcs: NonZeroUsize::new(1).unwrap(),
        },
    );

    let code =
        generate_gear_program_code(unstructured, gear_config).expect("failed wasm generation");
    let code = Code::try_new(code, 1, |_| CustomConstantCostRules::new(0, 0, 0), None)
        .expect("Failed to create Code");

    let code_id = CodeId::generate(code.original_code());
    let program_id = ProgramId::generate_from_user(code_id, b"");

    let mut message_context = MessageContext::new(
        IncomingDispatch::new(DispatchKind::Init, IncomingMessage::default(), None),
        program_id,
        ContextSettings::new(0, 0, 0, 0, 0, outgoing_limit),
    );

    if imitate_reply {
        let _ = message_context.reply_commit(ReplyPacket::auto(), None);
    }

    let processor_context = ProcessorContext {
        message_context,
        max_pages: INITIAL_PAGES.into(),
        rent_cost: 10,
        program_id,
        value_counter: ValueCounter::new(value),
        ..ProcessorContext::new_mock()
    };

    let ext = gear_core_processor::Ext::new(processor_context);
    let env = Environment::new(
        ext,
        code.code(),
        DispatchKind::Init,
        vec![DispatchKind::Init].into_iter().collect(),
        INITIAL_PAGES.into(),
    )
    .expect("Failed to create environment");

    let report = env
        .execute(|mem, _stack_end, globals_config| -> Result<(), u32> {
            gear_core_processor::Ext::lazy_pages_init_for_program(
                mem,
                program_id,
                Default::default(),
                Some(mem.size()),
                globals_config,
                Default::default(),
            );

            if let Some(mem_write) = initial_memory_write {
                return mem
                    .write(mem_write.offset, &mem_write.content)
                    .map_err(|_| 1);
            };

            Ok(())
        })
        .expect("Failed to execute WASM module");

    report
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    #[test]
    // Test that valid config always generates a valid gear wasm.
    fn test_standard_config(buf in prop::collection::vec(any::<u8>(), UNSTRUCTURED_SIZE)) {
        use gear_wasm_instrument::rules::CustomConstantCostRules;
        let mut u = Unstructured::new(&buf);
        let configs_bundle: StandardGearWasmConfigsBundle = StandardGearWasmConfigsBundle {
            log_info: Some("Some data".into()),
            entry_points_set: EntryPointsSet::InitHandleHandleReply,
            ..Default::default()
        };

        let original_code = generate_gear_program_code(&mut u, configs_bundle)
            .expect("failed generating wasm");

        let code_res = Code::try_new(original_code, 1, |_| CustomConstantCostRules::default(), None);
        assert!(code_res.is_ok());
    }

    #[test]
    fn test_reproduction(buf in prop::collection::vec(any::<u8>(), UNSTRUCTURED_SIZE)) {
        let mut u = Unstructured::new(&buf);
        let mut u2 = Unstructured::new(&buf);

        let gear_config = StandardGearWasmConfigsBundle::<[u8; 32]>::default();

        let first = generate_gear_program_code(&mut u, gear_config.clone()).expect("failed wasm generation");
        let second = generate_gear_program_code(&mut u2, gear_config).expect("failed wasm generation");

        assert_eq!(first, second);
    }
}
