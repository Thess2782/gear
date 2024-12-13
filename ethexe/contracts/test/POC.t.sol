// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.26;

import {console} from "forge-std/Test.sol";
import {MessageHashUtils} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import {Upgrades} from "openzeppelin-foundry-upgrades/Upgrades.sol";
import {EnumerableMap} from "@openzeppelin/contracts/utils/structs/EnumerableMap.sol";
import "forge-std/Test.sol";

import {POCBaseTest} from "symbiotic-core/test/POCBase.t.sol";

import {IVaultConfigurator} from "symbiotic-core/src/interfaces/IVaultConfigurator.sol";
import {IVault} from "symbiotic-core/src/interfaces/vault/IVault.sol";
import {IBaseDelegator} from "symbiotic-core/src/interfaces/delegator/IBaseDelegator.sol";
import {IOperatorSpecificDelegator} from "symbiotic-core/src/interfaces/delegator/IOperatorSpecificDelegator.sol";
import {IVetoSlasher} from "symbiotic-core/src/interfaces/slasher/IVetoSlasher.sol";
import {IBaseSlasher} from "symbiotic-core/src/interfaces/slasher/IBaseSlasher.sol";

import {Gear} from "../src/libraries/Gear.sol";
import {Base} from "./Base.t.sol";
import {MapWithTimeData} from "../src/libraries/MapWithTimeData.sol";
import {IMirror} from "../src/Mirror.sol";
import {IRouter} from "../src/IRouter.sol";
import {IMiddleware} from "./../src/IMiddleware.sol";

contract POCTest is Base {
    using MessageHashUtils for address;
    using EnumerableMap for EnumerableMap.AddressToUintMap;

    EnumerableMap.AddressToUintMap private operators;
    address[] private vaults;
    POCBaseTest private sym;

    function setUp() public override {
        admin = 0x116B4369a90d2E9DA6BD7a924A23B164E10f6FE9;
        eraDuration = 400;
        electionDuration = 100;
        blockDuration = 12;
        maxValidators = 3;

        setUpWrappedVara();

        setUpMiddleware();

        for (uint256 i = 0; i < 10; i++) {
            (address _addr, uint256 _key) = makeAddrAndKey(vm.toString(i + 1));
            operators.set(_addr, _key);
            address _vault = createOperatorWithStake(_addr, (i + 1) * 1000);
            vaults.push(_vault);
        }

        vm.warp(vm.getBlockTimestamp() + 1);
        address[] memory _validators = middleware.makeElectionAt(uint48(vm.getBlockTimestamp()) - 1, maxValidators);

        setUpRouter(_validators);

        // Change slash requester and executor to router
        // Note: just to check that it is possible to change them for now and do not affect the poc test
        vm.startPrank(admin);
        {
            middleware.changeSlashRequester(address(router));
            middleware.changeSlashExecutor(address(router));
        }
        vm.stopPrank();

        // For understanding where symbiotic ecosystem is using
        sym = POCBaseTest(address(this));
    }

    function test_POC() public {
        bytes32 _codeId = bytes32(uint256(1));
        bytes32 _blobTxHash = bytes32(uint256(2));

        router.requestCodeValidation(_codeId, _blobTxHash);

        address[] memory _validators = router.validators();
        assertEq(_validators.length, maxValidators);

        uint256[] memory _privateKeys = new uint256[](_validators.length);
        for (uint256 i = 0; i < _validators.length; i++) {
            address _operator = _validators[i];
            _privateKeys[i] = operators.get(_operator);
        }

        commitCode(_privateKeys, Gear.CodeCommitment(_codeId, true));

        address _ping = deployPing(_privateKeys, _codeId);
        IMirror actor = IMirror(_ping);
        assertEq(router.latestCommittedBlockHash(), blockHash(vm.getBlockNumber() - 1));
        assertEq(actor.stateHash(), bytes32(uint256(1)));
        assertEq(actor.nonce(), uint256(1));

        doPingPong(_privateKeys, _ping);
        assertEq(router.latestCommittedBlockHash(), blockHash(vm.getBlockNumber() - 1));
        assertEq(actor.stateHash(), bytes32(uint256(2)));
        assertEq(actor.nonce(), uint256(2));

        // Check that going to next era without re-election is ok and old validators are still valid.
        rollBlocks(eraDuration / blockDuration);
        doPingPong(_privateKeys, _ping);
        assertEq(router.latestCommittedBlockHash(), blockHash(vm.getBlockNumber() - 1));
        assertEq(actor.stateHash(), bytes32(uint256(2)));
        assertEq(actor.nonce(), uint256(3));

        // Change validators stake and make re-election
        depositInto(vaults[0], 10_000);
        depositInto(vaults[1], 10_000);
        depositInto(vaults[2], 10_000);
        rollBlocks((eraDuration - electionDuration) / blockDuration);
        _validators = middleware.makeElectionAt(uint48(vm.getBlockTimestamp()) - 1, maxValidators);

        commitValidators(_privateKeys, Gear.ValidatorsCommitment(_validators, 2));

        for (uint256 i = 0; i < _validators.length; i++) {
            address _operator = _validators[i];

            // Check that election is correct
            // Validators are sorted in descending order
            (address expected,) = makeAddrAndKey(vm.toString(_validators.length - i));
            assertEq(_operator, expected);

            _privateKeys[i] = operators.get(_operator);
        }

        // Go to a new era and commit from new validators
        rollBlocks(electionDuration / blockDuration);
        doPingPong(_privateKeys, _ping);
        assertEq(router.latestCommittedBlockHash(), blockHash(vm.getBlockNumber() - 1));
        assertEq(actor.stateHash(), bytes32(uint256(2)));
        assertEq(actor.nonce(), uint256(4));
    }

    function deployPing(uint256[] memory _privateKeys, bytes32 _codeId) private returns (address _ping) {
        vm.startPrank(admin, admin);
        {
            vm.expectEmit(true, false, false, false);
            emit IRouter.ProgramCreated(address(0), bytes32(uint256(1)));
            _ping = router.createProgram(_codeId, "salt");
            IMirror(_ping).sendMessage("PING", 0);
        }
        vm.stopPrank();

        uint48 _deploymentTimestamp = uint48(vm.getBlockTimestamp());
        bytes32 _deploymentBlock = blockHash(vm.getBlockNumber());

        rollBlocks(1);

        Gear.Message[] memory _outgoingMessages = new Gear.Message[](1);
        _outgoingMessages[0] = Gear.Message(
            0, // message id
            admin, // destination
            "PONG", // payload
            0, // value
            Gear.ReplyDetails(
                0, // reply to
                0 // reply code
            )
        );

        Gear.StateTransition[] memory _transitions = new Gear.StateTransition[](1);
        _transitions[0] = Gear.StateTransition(
            _ping, // actor id
            bytes32(uint256(1)), // new state hash
            address(0), // inheritor
            uint128(0), // value to receive
            new Gear.ValueClaim[](0), // value claims
            _outgoingMessages // messages
        );

        vm.expectEmit(true, false, false, false);
        emit IMirror.Message(0, admin, "PONG", 0);
        commitBlock(
            _privateKeys,
            Gear.BlockCommitment(
                _deploymentBlock, // commitment block hash
                _deploymentTimestamp, // commitment block timestamp
                router.latestCommittedBlockHash(), // previously committed block hash
                _deploymentBlock, // predecessor block hash
                _transitions // commitment transitions
            )
        );
    }

    function doPingPong(uint256[] memory _privateKeys, address _ping) internal {
        vm.startPrank(admin, admin);
        {
            uint256 _allowanceBefore = wrappedVara.allowance(admin, _ping);
            wrappedVara.approve(_ping, type(uint256).max);
            IMirror(_ping).sendMessage("PING", 0);
            wrappedVara.approve(_ping, _allowanceBefore);
        }
        vm.stopPrank();

        uint48 _pingTimestamp = uint48(vm.getBlockTimestamp());
        bytes32 _pingBlock = blockHash(vm.getBlockNumber());

        rollBlocks(1);

        Gear.Message[] memory _outgoingMessages = new Gear.Message[](1);
        _outgoingMessages[0] = Gear.Message(
            0, // message id
            admin, // destination
            "PONG", // payload
            0, // value
            Gear.ReplyDetails(
                0, // reply to
                0 // reply code
            )
        );

        Gear.StateTransition[] memory _transitions = new Gear.StateTransition[](1);
        _transitions[0] = Gear.StateTransition(
            _ping, // actor id
            bytes32(uint256(2)), // new state hash
            address(0), // inheritor
            0, // value to receive
            new Gear.ValueClaim[](0), // value claims
            _outgoingMessages // messages
        );

        vm.expectEmit(true, false, false, false);
        emit IMirror.Message(0, admin, "PONG", 0);
        commitBlock(
            _privateKeys,
            Gear.BlockCommitment(
                _pingBlock, // commitment block hash
                _pingTimestamp, // commitment block timestamp
                router.latestCommittedBlockHash(), // previously committed block hash
                _pingBlock, // predecessor block hash
                _transitions // commitment transitions
            )
        );
    }

    function test_requestSlash() public {
        address operator1 = address(0x1);
        address operator2 = address(0x2);

        uint256 stake1 = 1_000;
        uint256 stake2 = 2_000;

        address vault1 = createOperatorWithStake(operator1, stake1);
        address vault2 = createOperatorWithStake(operator2, stake2);

        rollBlocks(1);

        // Change validators stake and make re-election
        depositInto(vault1, 10_000);
        depositInto(vault2, 10_000);
        
        rollBlocks((eraDuration - electionDuration) / blockDuration);
        middleware.makeElectionAt(
            uint48(vm.getBlockTimestamp()) - 1,
            maxValidators
        );

        address middlewareAddress = address(middleware);
        
        // Middleware must be deployed and stored
        assertEq(
            router.middleware(),
            middlewareAddress,
            "Middleware address mismatch"
        );

        IMiddleware.VaultSlashData[]
            memory vaultData = new IMiddleware.VaultSlashData[](1);
        vaultData[0] = IMiddleware.VaultSlashData({
            vault: vault1,
            amount: 1000
        });

        // Prepare SlashData with the registered operator and timestamp
        IMiddleware.SlashData[]
            memory slashDataArray = new IMiddleware.SlashData[](1);
        slashDataArray[0] = IMiddleware.SlashData({
            operator: operator1, // The operator address
            ts: uint48(block.timestamp), // Current timestamp
            vaults: vaultData // Vault data prepared earlier
        });

        rollBlocks(1);
        
        vm.expectCall(
            middlewareAddress,
            abi.encodeWithSelector(
                IMiddleware.requestSlash.selector,
                slashDataArray
            )
        );

        router.requestSlashCommitment(slashDataArray);

        // IVault(vault1).sla

        console.log("requestSlashCommitment executed successfully.");
    }
}
