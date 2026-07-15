// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {ResourceAMM} from "../../src/exchange/ResourceAMM.sol";
import {ComputeToken} from "../../src/tokens/ComputeToken.sol";
import {TrainingToken} from "../../src/tokens/TrainingToken.sol";
import {BandwidthToken} from "../../src/tokens/BandwidthToken.sol";
import {StorageToken} from "../../src/tokens/StorageToken.sol";

contract ResourceAMMTest is Test {
    ResourceAMM private amm;
    ComputeToken private compute;
    TrainingToken private training;
    BandwidthToken private bandwidth;
    StorageToken private storageToken;

    address private trader = makeAddr("trader");
    address private outsider = makeAddr("outsider");

    uint256 private constant RESERVE = 1_000_000e18;

    function setUp() public {
        compute = new ComputeToken();
        training = new TrainingToken();
        bandwidth = new BandwidthToken();
        storageToken = new StorageToken();
        address[4] memory tokens =
            [address(compute), address(training), address(bandwidth), address(storageToken)];
        amm = new ResourceAMM(tokens, address(this));

        compute.mint(address(this), RESERVE * 7);
        training.mint(address(this), RESERVE * 7);
        bandwidth.mint(address(this), RESERVE * 7);
        storageToken.mint(address(this), RESERVE * 7);
        compute.mint(trader, 100_000e18);

        compute.approve(address(amm), type(uint256).max);
        training.approve(address(amm), type(uint256).max);
        bandwidth.approve(address(amm), type(uint256).max);
        storageToken.approve(address(amm), type(uint256).max);
        vm.prank(trader);
        compute.approve(address(amm), type(uint256).max);
    }

    function test_seedAllSixResourcePairs() public {
        address[4] memory tokens =
            [address(compute), address(training), address(bandwidth), address(storageToken)];
        uint256 count;
        for (uint256 i = 0; i < tokens.length; i++) {
            for (uint256 j = i + 1; j < tokens.length; j++) {
                amm.seedPool(tokens[i], tokens[j], RESERVE, RESERVE);
                (uint256 reserveA, uint256 reserveB) = amm.getReserves(tokens[i], tokens[j]);
                assertEq(reserveA, RESERVE);
                assertEq(reserveB, RESERVE);
                count++;
            }
        }
        assertEq(count, 6);
    }

    function test_quoteAndSwapMatchRustReferenceVector() public {
        amm.seedPool(address(compute), address(storageToken), 1_000_000, 1_000_000);
        compute.mint(trader, 10_000);
        vm.prank(trader);
        compute.approve(address(amm), type(uint256).max);

        assertEq(amm.quoteExactInput(address(compute), address(storageToken), 10_000), 9_871);
        vm.prank(trader);
        uint256 amountOut = amm.swapExactInput(
            address(compute), address(storageToken), 10_000, 9_800, trader, block.timestamp
        );

        assertEq(amountOut, 9_871);
        assertEq(storageToken.balanceOf(trader), 9_871);
        (uint256 reserveIn, uint256 reserveOut) =
            amm.getReserves(address(compute), address(storageToken));
        assertEq(reserveIn, 1_010_000);
        assertEq(reserveOut, 990_129);
        assertGe(reserveIn * reserveOut, 1_000_000 * 1_000_000);
    }

    function test_reverseDirectionUsesCanonicalPool() public {
        amm.seedPool(address(compute), address(storageToken), RESERVE, RESERVE / 2);
        storageToken.mint(trader, 10_000e18);
        vm.prank(trader);
        storageToken.approve(address(amm), type(uint256).max);

        uint256 quote = amm.quoteExactInput(address(storageToken), address(compute), 1_000e18);
        vm.prank(trader);
        assertEq(
            amm.swapExactInput(
                address(storageToken), address(compute), 1_000e18, quote, trader, block.timestamp
            ),
            quote
        );
    }

    function test_addLiquidityRequiresCurrentRatio() public {
        amm.seedPool(address(compute), address(training), RESERVE, RESERVE / 2);
        amm.addLiquidity(address(compute), address(training), 100e18, 50e18);
        (uint256 reserveCompute, uint256 reserveTraining) =
            amm.getReserves(address(compute), address(training));
        assertEq(reserveCompute, RESERVE + 100e18);
        assertEq(reserveTraining, RESERVE / 2 + 50e18);

        vm.expectRevert(ResourceAMM.InsufficientLiquidity.selector);
        amm.addLiquidity(address(compute), address(training), 100e18, 51e18);
    }

    function test_revertsForSlippageDeadlineAndUnauthorizedSeed() public {
        amm.seedPool(address(compute), address(storageToken), RESERVE, RESERVE);

        uint256 quote = amm.quoteExactInput(address(compute), address(storageToken), 1_000e18);
        vm.prank(trader);
        vm.expectRevert(
            abi.encodeWithSelector(
                ResourceAMM.InsufficientOutput.selector, type(uint256).max, quote
            )
        );
        amm.swapExactInput(
            address(compute),
            address(storageToken),
            1_000e18,
            type(uint256).max,
            trader,
            block.timestamp
        );

        vm.prank(trader);
        vm.expectRevert(
            abi.encodeWithSelector(
                ResourceAMM.DeadlineExpired.selector, block.timestamp - 1, block.timestamp
            )
        );
        amm.swapExactInput(
            address(compute), address(storageToken), 1_000e18, 0, trader, block.timestamp - 1
        );

        vm.prank(outsider);
        vm.expectRevert();
        amm.seedPool(address(compute), address(training), 1, 1);
    }

    function testFuzz_swapNeverDecreasesInvariant(uint96 rawAmountIn) public {
        amm.seedPool(address(compute), address(storageToken), RESERVE, RESERVE);
        uint256 amountIn = bound(uint256(rawAmountIn), 1e18, 100_000e18);
        compute.mint(trader, amountIn);
        uint256 invariantBefore = RESERVE * RESERVE;

        vm.prank(trader);
        amm.swapExactInput(
            address(compute), address(storageToken), amountIn, 0, trader, block.timestamp
        );

        (uint256 reserveA, uint256 reserveB) =
            amm.getReserves(address(compute), address(storageToken));
        assertGe(reserveA * reserveB, invariantBefore);
    }
}
