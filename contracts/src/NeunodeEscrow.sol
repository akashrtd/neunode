// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title NeunodeEscrow — Bilateral escrow for bounty payments
/// @notice iExec-style escrow: requester deposits payment, provider bonds 15%,
///         release on accept, refund on reject, dispute resolution placeholder.
contract NeunodeEscrow {
    using SafeERC20 for IERC20;

    // ─── Types ────────────────────────────────────────────────────────────

    enum EscrowState {
        Created,   // Deposit held, waiting for provider
        Funded,    // Provider bonded, work in progress
        Completed, // Funds released to provider
        Refunded,  // Funds returned to requester
        Disputed   // Under dispute resolution
    }

    struct Escrow {
        bytes32 bountyId;
        address requester;
        address provider;
        address token;           // ERC-20 token address
        uint256 amount;          // Payment amount
        uint256 providerBond;    // 15% bond from provider
        uint256 created;
        uint256 deadline;        // Work deadline
        EscrowState state;
    }

    // ─── Storage ──────────────────────────────────────────────────────────

    mapping(bytes32 => Escrow) public escrows;

    uint256 public constant PROVIDER_BOND_BPS = 1500; // 15% in basis points

    // ─── Events ───────────────────────────────────────────────────────────

    event EscrowCreated(
        bytes32 indexed bountyId, address indexed requester, address token, uint256 amount
    );
    event EscrowFunded(
        bytes32 indexed bountyId, address indexed provider, uint256 bond
    );
    event EscrowReleased(
        bytes32 indexed bountyId, address indexed provider, uint256 amount
    );
    event EscrowRefunded(
        bytes32 indexed bountyId, address indexed requester, uint256 amount
    );
    event EscrowDisputed(bytes32 indexed bountyId, uint256 timestamp);

    // ─── Errors ───────────────────────────────────────────────────────────

    error EscrowNotFound(bytes32 bountyId);
    error EscrowNotCreated(bytes32 bountyId);
    error EscrowNotFunded(bytes32 bountyId);
    error NotRequester(bytes32 bountyId, address caller);
    error NotProvider(bytes32 bountyId, address caller);
    error InvalidAmount();
    error InvalidToken();
    error DeadlinePassed(uint256 deadline);
    error TransferFailed();

    // ─── Functions ────────────────────────────────────────────────────────

    /// @notice Create escrow — requester deposits payment tokens
    function createEscrow(
        bytes32 bountyId,
        address token,
        uint256 amount,
        uint256 deadline
    ) external {
        if (amount == 0) revert InvalidAmount();
        if (token == address(0)) revert InvalidToken();
        if (escrows[bountyId].created != 0) revert EscrowNotFound(bountyId); // reentrancy guard

        // Transfer payment from requester to this contract
        bool success = IERC20(token).transferFrom(msg.sender, address(this), amount);
        if (!success) revert TransferFailed();

        escrows[bountyId] = Escrow({
            bountyId: bountyId,
            requester: msg.sender,
            provider: address(0),
            token: token,
            amount: amount,
            providerBond: 0,
            created: block.timestamp,
            deadline: deadline,
            state: EscrowState.Created
        });

        emit EscrowCreated(bountyId, msg.sender, token, amount);
    }

    /// @notice Provider bonds 15% and accepts the escrow
    function fundEscrow(bytes32 bountyId, uint256 providerBond) external {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Created) revert EscrowNotCreated(bountyId);

        // Validate bond is at least 15% of amount
        uint256 minBond = (escrow.amount * PROVIDER_BOND_BPS) / 10_000;
        if (providerBond < minBond) revert InvalidAmount();

        // Transfer bond from provider
        bool success = IERC20(escrow.token).transferFrom(msg.sender, address(this), providerBond);
        if (!success) revert TransferFailed();

        escrow.provider = msg.sender;
        escrow.providerBond = providerBond;
        escrow.state = EscrowState.Funded;

        emit EscrowFunded(bountyId, msg.sender, providerBond);
    }

    /// @notice Release payment to provider (requester accepts work)
    function release(bytes32 bountyId) external {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);
        if (escrow.requester != msg.sender) revert NotRequester(bountyId, msg.sender);

        uint256 totalPayout = escrow.amount + escrow.providerBond;
        escrow.state = EscrowState.Completed;

        IERC20(escrow.token).safeTransfer(escrow.provider, totalPayout);

        emit EscrowReleased(bountyId, escrow.provider, totalPayout);
    }

    /// @notice Refund payment to requester (work rejected)
    function refund(bytes32 bountyId) external {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);
        if (escrow.requester != msg.sender) revert NotRequester(bountyId, msg.sender);

        uint256 refundAmount = escrow.amount;
        uint256 bondSlashed = escrow.providerBond;
        escrow.state = EscrowState.Refunded;

        // Refund requester
        IERC20(escrow.token).safeTransfer(escrow.requester, refundAmount);
        // Slash provider bond to requester
        IERC20(escrow.token).safeTransfer(escrow.requester, bondSlashed);

        emit EscrowRefunded(bountyId, escrow.requester, refundAmount + bondSlashed);
    }

    /// @notice Either party can dispute
    function dispute(bytes32 bountyId) external {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);
        if (escrow.requester != msg.sender && escrow.provider != msg.sender) {
            revert NotProvider(bountyId, msg.sender);
        }

        escrow.state = EscrowState.Disputed;

        emit EscrowDisputed(bountyId, block.timestamp);
    }

    /// @notice Get escrow state
    function getEscrowState(bytes32 bountyId) external view returns (EscrowState) {
        if (escrows[bountyId].created == 0) revert EscrowNotFound(bountyId);
        return escrows[bountyId].state;
    }
}
