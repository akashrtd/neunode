// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {Deploy} from "../script/Deploy.s.sol";
import {NeunodeSlashing} from "../src/slashing/NeunodeSlashing.sol";

contract DeploymentTopologyTest is Test {
    uint256 private constant DEPLOYER_KEY = 0xA11CE;
    uint256 private constant STAKE = 20_000e18;

    Deploy private deployment;
    address private deployer;

    function setUp() public {
        deployer = vm.addr(DEPLOYER_KEY);
        vm.setEnv("PRIVATE_KEY", vm.toString(DEPLOYER_KEY));
        deployment = new Deploy();
        deployment.run();
    }

    function test_deploysAndConnectsEveryProtocolModule() public view {
        assertGt(address(deployment.reputation()).code.length, 0);
        assertGt(address(deployment.slashing()).code.length, 0);
        assertGt(address(deployment.stakingEscrow()).code.length, 0);

        assertEq(address(deployment.registry().identity()), address(deployment.identity()));
        assertEq(address(deployment.identity().stakeSource()), address(deployment.computeToken()));
        assertEq(
            address(deployment.reputation().identityRegistry()), address(deployment.identity())
        );
        assertEq(address(deployment.reputation().stakeSource()), address(deployment.computeToken()));
        assertEq(address(deployment.slashing().token()), address(deployment.computeToken()));
        assertEq(address(deployment.slashing().reputation()), address(deployment.reputation()));
        assertEq(
            address(deployment.stakingEscrow().neunodeToken()), address(deployment.computeToken())
        );
        assertEq(address(deployment.bounty().escrow()), address(deployment.escrow()));
        assertEq(address(deployment.bounty().reviewContract()), address(deployment.review()));
        assertEq(
            address(deployment.royaltySplitter().registry()), address(deployment.modelRegistry())
        );
    }

    function test_assignsOperationalAndGovernancePermissions() public view {
        address governance = address(deployment.governance());

        assertEq(deployment.identity().owner(), governance);
        assertTrue(
            deployment.computeToken()
                .hasRole(
                    deployment.computeToken().GOVERNANCE_ROLE(), address(deployment.slashing())
                )
        );
        assertTrue(
            deployment.computeToken()
                .hasRole(
                    deployment.computeToken().GOVERNANCE_ROLE(), address(deployment.stakingEscrow())
                )
        );
        assertTrue(
            deployment.reputation()
                .hasRole(deployment.reputation().SLASHING_ROLE(), address(deployment.slashing()))
        );
        assertTrue(
            deployment.escrow()
                .hasRole(deployment.escrow().BOUNTY_CONTRACT_ROLE(), address(deployment.bounty()))
        );
        assertTrue(
            deployment.review()
                .hasRole(deployment.review().DEFAULT_ADMIN_ROLE(), address(deployment.bounty()))
        );
        assertTrue(
            deployment.reputation()
                .hasRole(deployment.reputation().ACTIVITY_ORACLE_ROLE(), deployer)
        );
        assertTrue(deployment.slashing().hasRole(deployment.slashing().REPORTER_ROLE(), deployer));

        assertTrue(deployment.governance().allowedTargets(address(deployment.identity())));
        assertTrue(deployment.governance().allowedTargets(address(deployment.reputation())));
        assertTrue(deployment.governance().allowedTargets(address(deployment.slashing())));
        assertTrue(deployment.governance().allowedTargets(address(deployment.stakingEscrow())));
        assertTrue(deployment.governance().allowedTargets(address(deployment.bounty())));
    }

    function test_slashingAtomicallyUpdatesStakeAndReputation() public {
        address validator = makeAddr("validator");

        vm.startPrank(deployer);
        deployment.computeToken().mint(validator, STAKE);
        deployment.reputation().updateFactorScore(validator, 1, 10_000);
        deployment.reputation().updateFactorScore(validator, 2, 10_000);
        deployment.reputation().updateFactorScore(validator, 3, 10_000);
        deployment.reputation().updateFactorScore(validator, 4, 10_000);
        vm.stopPrank();

        vm.startPrank(validator);
        deployment.computeToken().stake(STAKE);
        vm.stopPrank();
        deployment.reputation().deriveStakeFactor(validator);

        uint256 stakeBefore = deployment.computeToken().stakedBalanceOf(validator);
        uint256 reputationBefore = deployment.reputation().getCompositeScore(validator);

        deployment.slashing().reportDowntime(validator, 51, 100);

        assertLt(deployment.computeToken().stakedBalanceOf(validator), stakeBefore);
        assertLt(deployment.reputation().getCompositeScore(validator), reputationBefore);
        assertEq(
            deployment.slashing().getOffenseCount(validator, NeunodeSlashing.OffenseType.Downtime),
            1
        );
    }
}
