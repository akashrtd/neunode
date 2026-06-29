// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "./bounty/IBountyEscrow.sol";

/// @title NeunodeEscrow — Bilateral escrow for bounty payments
/// @notice iExec-style escrow: requester deposits payment, provider bonds 15%,
///         release on accept, refund on reject, dispute resolution placeholder.
///         Now integrates with bounty lifecycle via IBountyEscrow interface.
contract NeunodeEscrow is IBountyEscrow, AccessControl, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ─── Types ────────────────────────────────────────────────────────────

    enum EscrowState {
        Created, // Deposit held, waiting for provider
        Funded, // Provider bonded, work in progress
        Completed, // Funds released to provider
        Refunded, // Funds returned to requester
        Disputed // Under dispute resolution
    }

    // NOTE: Escrow struct kept at original 9 fields to preserve tuple layout
    // for existing test destructuring. New fields in separate mappings below.
    struct Escrow {
        bytes32 bountyId;
        address requester;
        address provider;
        address token; // ERC-20 token address
        uint256 amount; // Payment amount
        uint256 providerBond; // 15% bond from provider
        uint256 created;
        uint256 deadline; // Work deadline
        EscrowState state;
    }

    // ─── Storage ──────────────────────────────────────────────────────────

    mapping(bytes32 => Escrow) public escrows;

    // New fields in separate mappings (preserves existing tuple layout)
    mapping(bytes32 => address) public escrowBountyContracts;

    uint256 public constant PROVIDER_BOND_BPS = 1500; // 15% in basis points

    bytes32 public constant ESCROW_ADMIN_ROLE = keccak256("ESCROW_ADMIN_ROLE");
    bytes32 public constant BOUNTY_CONTRACT_ROLE = keccak256("BOUNTY_CONTRACT_ROLE");

    // ─── Events ───────────────────────────────────────────────────────────

    event EscrowCreated(
        bytes32 indexed bountyId, address indexed requester, address token, uint256 amount
    );
    event EscrowFunded(bytes32 indexed bountyId, address indexed provider, uint256 bond);
    event EscrowReleased(bytes32 indexed bountyId, address indexed provider, uint256 amount);
    event EscrowRefunded(bytes32 indexed bountyId, address indexed requester, uint256 amount);
    event EscrowDisputed(bytes32 indexed bountyId, uint256 timestamp);
    event EscrowReleasedWithFees(
        bytes32 indexed bountyId,
        address indexed provider,
        uint256 providerPayout,
        uint256 protocolFee,
        uint256 reviewerFee,
        uint256 verificationFee
    );
    event BountyContractRegistered(address indexed bountyContract);

    // ─── Errors ───────────────────────────────────────────────────────────

    error EscrowNotFound(bytes32 bountyId);
    error EscrowAlreadyExists(bytes32 bountyId);
    error EscrowNotCreated(bytes32 bountyId);
    error EscrowNotFunded(bytes32 bountyId);
    error NotRequester(bytes32 bountyId, address caller);
    error NotProvider(bytes32 bountyId, address caller);
    error InvalidAmount();
    error InvalidToken();
    error DeadlinePassed(uint256 deadline);
    error Unauthorized();
    error FeeBpsExceeds100Pct(uint256 totalBps);
    error ZeroAddressFeeRecipient();

    // ─── Constructor ──────────────────────────────────────────────────────

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(ESCROW_ADMIN_ROLE, msg.sender);
    }

    // ─── Admin Functions ──────────────────────────────────────────────────

    /// @notice Register a bounty contract that can call IBountyEscrow methods
    function registerBountyContract(address bountyContract) external onlyRole(ESCROW_ADMIN_ROLE) {
        _grantRole(BOUNTY_CONTRACT_ROLE, bountyContract);
        emit BountyContractRegistered(bountyContract);
    }

    // ─── IBountyEscrow Implementation ─────────────────────────────────────

    /// @notice Create escrow tied to a bounty (called by bounty contract)
    function createBountyEscrow(
        bytes32 bountyId,
        address requester_,
        address token,
        uint256 amount,
        uint256 workDeadline
    ) external override onlyRole(BOUNTY_CONTRACT_ROLE) {
        if (amount == 0) revert InvalidAmount();
        if (token == address(0)) revert InvalidToken();
        if (escrows[bountyId].created != 0) revert EscrowAlreadyExists(bountyId);

        // Transfer payment from requester to this contract
        IERC20(token).safeTransferFrom(requester_, address(this), amount);

        escrows[bountyId] = Escrow({
            bountyId: bountyId,
            requester: requester_,
            provider: address(0),
            token: token,
            amount: amount,
            providerBond: 0,
            created: block.timestamp,
            deadline: workDeadline,
            state: EscrowState.Created
        });
        escrowBountyContracts[bountyId] = msg.sender;

        emit EscrowCreated(bountyId, requester_, token, amount);
    }

    /// @notice Provider bonds when claiming (called by bounty contract)
    function bondProvider(bytes32 bountyId, address provider_, uint256 bondAmount)
        external
        override
        onlyRole(BOUNTY_CONTRACT_ROLE)
    {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Created) revert EscrowNotCreated(bountyId);

        uint256 minBond = (escrow.amount * PROVIDER_BOND_BPS) / 10_000;
        if (bondAmount < minBond) revert InvalidAmount();

        IERC20(escrow.token).safeTransferFrom(provider_, address(this), bondAmount);

        escrow.provider = provider_;
        escrow.providerBond = bondAmount;
        escrow.state = EscrowState.Funded;

        emit EscrowFunded(bountyId, provider_, bondAmount);
    }

    /// @notice Release with fee splitting (called by bounty contract)
    function releaseWithFees(
        bytes32 bountyId,
        address provider_,
        uint256 protocolFeeBps,
        uint256 reviewerFeeBps,
        uint256 verificationFeeBps,
        address protocolFeeRecipient,
        address reviewerFeeRecipient,
        address verificationFeeRecipient
    ) external override onlyRole(BOUNTY_CONTRACT_ROLE) nonReentrant {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);

        uint256 totalFeesBps = protocolFeeBps + reviewerFeeBps + verificationFeeBps;
        if (totalFeesBps > 10_000) revert FeeBpsExceeds100Pct(totalFeesBps);

        if (protocolFeeBps > 0 && protocolFeeRecipient == address(0)) {
            revert ZeroAddressFeeRecipient();
        }
        if (reviewerFeeBps > 0 && reviewerFeeRecipient == address(0)) {
            revert ZeroAddressFeeRecipient();
        }
        if (verificationFeeBps > 0 && verificationFeeRecipient == address(0)) {
            revert ZeroAddressFeeRecipient();
        }
        uint256 totalFee = (escrow.amount * totalFeesBps) / 10_000;
        uint256 providerPayout = escrow.amount - totalFee;

        escrow.state = EscrowState.Completed;

        // Distribute fees
        uint256 protocolFee;
        uint256 reviewerFee;
        uint256 verificationFee;

        if (protocolFeeBps > 0) {
            protocolFee = (escrow.amount * protocolFeeBps) / 10_000;
            IERC20(escrow.token).safeTransfer(protocolFeeRecipient, protocolFee);
        }
        if (reviewerFeeBps > 0) {
            reviewerFee = (escrow.amount * reviewerFeeBps) / 10_000;
            IERC20(escrow.token).safeTransfer(reviewerFeeRecipient, reviewerFee);
        }
        if (verificationFeeBps > 0) {
            verificationFee = (escrow.amount * verificationFeeBps) / 10_000;
            IERC20(escrow.token).safeTransfer(verificationFeeRecipient, verificationFee);
        }

        // Pay provider (payout + bond)
        IERC20(escrow.token).safeTransfer(provider_, providerPayout + escrow.providerBond);

        emit EscrowReleasedWithFees(
            bountyId, provider_, providerPayout, protocolFee, reviewerFee, verificationFee
        );
        emit EscrowReleased(bountyId, provider_, providerPayout + escrow.providerBond);
    }

    /// @notice Refund requester (called by bounty contract)
    function refundRequester(bytes32 bountyId)
        external
        override
        onlyRole(BOUNTY_CONTRACT_ROLE)
        nonReentrant
    {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);

        uint256 refundAmount = escrow.amount;
        uint256 bondSlashed = escrow.providerBond;
        escrow.state = EscrowState.Refunded;

        // Refund requester (amount + slashed bond combined into single transfer)
        IERC20(escrow.token).safeTransfer(escrow.requester, refundAmount + bondSlashed);

        emit EscrowRefunded(bountyId, escrow.requester, refundAmount + bondSlashed);
    }

    /// @notice Check if escrow exists and is funded
    function isEscrowFunded(bytes32 bountyId) external view override returns (bool) {
        return escrows[bountyId].created != 0 && escrows[bountyId].state == EscrowState.Funded;
    }

    // ─── Direct Escrow Functions (backward-compatible) ────────────────────

    /// @notice Create escrow — requester deposits payment tokens
    function createEscrow(bytes32 bountyId, address token, uint256 amount, uint256 deadline)
        external
    {
        if (amount == 0) revert InvalidAmount();
        if (token == address(0)) revert InvalidToken();
        if (escrows[bountyId].created != 0) revert EscrowAlreadyExists(bountyId);

        // Transfer payment from requester to this contract
        IERC20(token).safeTransferFrom(msg.sender, address(this), amount);

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
        IERC20(escrow.token).safeTransferFrom(msg.sender, address(this), providerBond);

        escrow.provider = msg.sender;
        escrow.providerBond = providerBond;
        escrow.state = EscrowState.Funded;

        emit EscrowFunded(bountyId, msg.sender, providerBond);
    }

    /// @notice Release payment to provider (requester accepts work)
    function release(bytes32 bountyId) external nonReentrant {
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
    function refund(bytes32 bountyId) external nonReentrant {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);
        if (escrow.requester != msg.sender) revert NotRequester(bountyId, msg.sender);

        uint256 refundAmount = escrow.amount;
        uint256 bondSlashed = escrow.providerBond;
        escrow.state = EscrowState.Refunded;

        // Refund requester (amount + slashed bond combined into single transfer)
        IERC20(escrow.token).safeTransfer(escrow.requester, refundAmount + bondSlashed);

        emit EscrowRefunded(bountyId, escrow.requester, refundAmount + bondSlashed);
    }

    /// @notice Either party can dispute
    function dispute(bytes32 bountyId) external nonReentrant {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);
        if (escrow.requester != msg.sender && escrow.provider != msg.sender) {
            revert NotProvider(bountyId, msg.sender);
        }

        escrow.state = EscrowState.Disputed;

        emit EscrowDisputed(bountyId, block.timestamp);
    }

    /// @notice Auto-refund after inactivity timeout (callable by anyone)
    function autoRefund(bytes32 bountyId, uint256 timeoutSeconds) external nonReentrant {
        Escrow storage escrow = escrows[bountyId];
        if (escrow.created == 0) revert EscrowNotFound(bountyId);
        if (escrow.state != EscrowState.Funded) revert EscrowNotFunded(bountyId);

        if (block.timestamp < escrow.deadline + timeoutSeconds) {
            revert DeadlinePassed(escrow.deadline + timeoutSeconds);
        }

        uint256 refundAmount = escrow.amount;
        uint256 bondReturn = escrow.providerBond;
        escrow.state = EscrowState.Refunded;

        IERC20(escrow.token).safeTransfer(escrow.requester, refundAmount);
        IERC20(escrow.token).safeTransfer(escrow.provider, bondReturn);

        emit EscrowRefunded(bountyId, escrow.requester, refundAmount);
    }

    /// @notice Get escrow state
    function getEscrowState(bytes32 bountyId) external view returns (EscrowState) {
        if (escrows[bountyId].created == 0) revert EscrowNotFound(bountyId);
        return escrows[bountyId].state;
    }
}
