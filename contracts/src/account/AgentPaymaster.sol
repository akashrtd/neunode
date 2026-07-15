// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {IEntryPoint, PackedUserOperation} from "./IEntryPoint.sol";

/// @title AgentPaymaster
/// @notice ERC-4337 verifying paymaster for policy-approved agent operations.
/// @dev The off-chain sponsor must inspect the complete UserOperation before signing. Signatures bind
///      the EntryPoint-provided operation hash, chain, paymaster, validity window, and maximum cost.
contract AgentPaymaster is AccessControl, Pausable, EIP712 {
    bytes32 public constant SPONSOR_ADMIN_ROLE = keccak256("SPONSOR_ADMIN_ROLE");
    bytes32 public constant SPONSORSHIP_TYPEHASH = keccak256(
        "Sponsorship(bytes32 userOpHash,uint256 sponsorLimit,uint48 validUntil,uint48 validAfter)"
    );
    uint256 public constant PAYMASTER_DATA_OFFSET = 52;
    bytes8 public constant PAYMASTER_SIG_MAGIC = 0x22e325a297439656;
    uint256 private constant POLICY_DATA_LENGTH = 96;
    uint256 private constant SIGNATURE_SUFFIX_LENGTH = 10;
    uint256 private constant SIG_VALIDATION_FAILED = 1;

    IEntryPoint public immutable entryPoint;
    address public sponsorSigner;

    error OnlyEntryPoint();
    error ZeroAddress();
    error InvalidPaymasterData();
    error SponsorLimitExceeded(uint256 maxCost, uint256 sponsorLimit);

    event SponsorSignerUpdated(address indexed previousSigner, address indexed newSigner);
    event EntryPointDepositWithdrawn(address indexed recipient, uint256 amount);

    constructor(address entryPoint_, address sponsorSigner_, address admin)
        EIP712("Neunode Agent Paymaster", "1")
    {
        if (entryPoint_ == address(0) || sponsorSigner_ == address(0) || admin == address(0)) {
            revert ZeroAddress();
        }
        entryPoint = IEntryPoint(entryPoint_);
        sponsorSigner = sponsorSigner_;
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(SPONSOR_ADMIN_ROLE, admin);
    }

    modifier onlyEntryPoint() {
        if (msg.sender != address(entryPoint)) revert OnlyEntryPoint();
        _;
    }

    /// @notice Validates a sponsor authorization encoded after the standard 52-byte paymaster prefix.
    /// @return context Empty because this policy requires no post-operation accounting.
    /// @return validationData ERC-4337 packed signature status and validity window.
    function validatePaymasterUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    )
        external
        view
        onlyEntryPoint
        whenNotPaused
        returns (bytes memory context, uint256 validationData)
    {
        if (
            userOp.paymasterAndData.length
                < PAYMASTER_DATA_OFFSET + POLICY_DATA_LENGTH + SIGNATURE_SUFFIX_LENGTH
        ) {
            revert InvalidPaymasterData();
        }
        address encodedPaymaster = address(bytes20(userOp.paymasterAndData[0:20]));
        if (encodedPaymaster != address(this)) revert InvalidPaymasterData();

        (uint48 validUntil, uint48 validAfter, uint256 sponsorLimit) = abi.decode(
            userOp.paymasterAndData[PAYMASTER_DATA_OFFSET:PAYMASTER_DATA_OFFSET + POLICY_DATA_LENGTH
            ],
            (uint48, uint48, uint256)
        );
        bytes memory signature = _decodePaymasterSignature(userOp.paymasterAndData);
        if (maxCost > sponsorLimit) revert SponsorLimitExceeded(maxCost, sponsorLimit);

        bytes32 digest = getSponsorshipHash(userOpHash, sponsorLimit, validUntil, validAfter);
        (address recovered, ECDSA.RecoverError error,) = ECDSA.tryRecover(digest, signature);
        uint256 signatureStatus = error == ECDSA.RecoverError.NoError && recovered == sponsorSigner
            ? 0
            : SIG_VALIDATION_FAILED;
        return ("", _packValidationData(signatureStatus, validUntil, validAfter));
    }

    /// @notice ERC-4337 callback. This paymaster returns empty context, so compliant EntryPoints do
    ///         not call it; the guarded no-op remains for interface compatibility.
    function postOp(uint8, bytes calldata, uint256, uint256) external view onlyEntryPoint {}

    function getSponsorshipHash(
        bytes32 userOpHash,
        uint256 sponsorLimit,
        uint48 validUntil,
        uint48 validAfter
    ) public view returns (bytes32) {
        return _hashTypedDataV4(
            keccak256(
                abi.encode(SPONSORSHIP_TYPEHASH, userOpHash, sponsorLimit, validUntil, validAfter)
            )
        );
    }

    function setSponsorSigner(address newSigner) external onlyRole(SPONSOR_ADMIN_ROLE) {
        if (newSigner == address(0)) revert ZeroAddress();
        emit SponsorSignerUpdated(sponsorSigner, newSigner);
        sponsorSigner = newSigner;
    }

    function pause() external onlyRole(SPONSOR_ADMIN_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(SPONSOR_ADMIN_ROLE) {
        _unpause();
    }

    function deposit() external payable onlyRole(SPONSOR_ADMIN_ROLE) {
        entryPoint.depositTo{value: msg.value}(address(this));
    }

    function depositBalance() external view returns (uint256) {
        return entryPoint.balanceOf(address(this));
    }

    function withdrawDeposit(address payable recipient, uint256 amount)
        external
        onlyRole(SPONSOR_ADMIN_ROLE)
    {
        if (recipient == address(0)) revert ZeroAddress();
        entryPoint.withdrawTo(recipient, amount);
        emit EntryPointDepositWithdrawn(recipient, amount);
    }

    function addStake(uint32 unstakeDelaySec) external payable onlyRole(SPONSOR_ADMIN_ROLE) {
        entryPoint.addStake{value: msg.value}(unstakeDelaySec);
    }

    function unlockStake() external onlyRole(SPONSOR_ADMIN_ROLE) {
        entryPoint.unlockStake();
    }

    function withdrawStake(address payable recipient) external onlyRole(SPONSOR_ADMIN_ROLE) {
        if (recipient == address(0)) revert ZeroAddress();
        entryPoint.withdrawStake(recipient);
    }

    function _packValidationData(uint256 signatureStatus, uint48 validUntil, uint48 validAfter)
        private
        pure
        returns (uint256)
    {
        return signatureStatus | (uint256(validUntil) << 160) | (uint256(validAfter) << 208);
    }

    function _decodePaymasterSignature(bytes calldata paymasterAndData)
        private
        pure
        returns (bytes memory signature)
    {
        uint256 suffixStart = paymasterAndData.length - SIGNATURE_SUFFIX_LENGTH;
        uint16 declaredLength = uint16(bytes2(paymasterAndData[suffixStart:suffixStart + 2]));
        if (bytes8(paymasterAndData[suffixStart + 2:]) != PAYMASTER_SIG_MAGIC) {
            revert InvalidPaymasterData();
        }
        uint256 signatureStart = PAYMASTER_DATA_OFFSET + POLICY_DATA_LENGTH;
        if (declaredLength != suffixStart - signatureStart) revert InvalidPaymasterData();
        return paymasterAndData[signatureStart:suffixStart];
    }
}
