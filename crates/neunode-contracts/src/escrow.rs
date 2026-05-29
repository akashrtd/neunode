//! NeunodeEscrow contract bindings.
//!
//! Bilateral escrow for bounty payments. iExec-style escrow: requester deposits
//! payment, provider bonds 15%, release on accept, refund on reject.

use alloy::sol;

sol! {
    // ─── Enums ────────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq, Eq)]
    enum EscrowState {
        Created,
        Funded,
        Completed,
        Refunded,
        Disputed
    }

    // ─── Structs ──────────────────────────────────────────────────────────

    #[derive(Debug, PartialEq)]
    struct Escrow {
        bytes32 bountyId;
        address requester;
        address provider;
        address token;
        uint256 amount;
        uint256 providerBond;
        uint256 created;
        uint256 deadline;
        EscrowState state;
    }

    // ─── Events ───────────────────────────────────────────────────────────

    #[derive(Debug)]
    event EscrowCreated(bytes32 indexed bountyId, address indexed requester, address token, uint256 amount);

    #[derive(Debug)]
    event EscrowFunded(bytes32 indexed bountyId, address indexed provider, uint256 bond);

    #[derive(Debug)]
    event EscrowReleased(bytes32 indexed bountyId, address indexed provider, uint256 amount);

    #[derive(Debug)]
    event EscrowRefunded(bytes32 indexed bountyId, address indexed requester, uint256 amount);

    #[derive(Debug)]
    event EscrowDisputed(bytes32 indexed bountyId, uint256 timestamp);

    #[derive(Debug)]
    event EscrowReleasedWithFees(
        bytes32 indexed bountyId,
        address indexed provider,
        uint256 providerPayout,
        uint256 protocolFee,
        uint256 reviewerFee,
        uint256 verificationFee
    );

    #[derive(Debug)]
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

    // ─── Functions ────────────────────────────────────────────────────────

    // Admin
    function registerBountyContract(address bountyContract) external;

    // IBountyEscrow implementation (called by bounty contract)
    function createBountyEscrow(
        bytes32 bountyId,
        address requester_,
        address token,
        uint256 amount,
        uint256 workDeadline
    ) external;

    function bondProvider(bytes32 bountyId, address provider_, uint256 bondAmount) external;

    function releaseWithFees(
        bytes32 bountyId,
        address provider_,
        uint256 protocolFeeBps,
        uint256 reviewerFeeBps,
        uint256 verificationFeeBps,
        address protocolFeeRecipient,
        address reviewerFeeRecipient,
        address verificationFeeRecipient
    ) external;

    function refundRequester(bytes32 bountyId) external;
    function isEscrowFunded(bytes32 bountyId) external view returns (bool);

    // Direct escrow functions
    function createEscrow(bytes32 bountyId, address token, uint256 amount, uint256 deadline) external;
    function fundEscrow(bytes32 bountyId, uint256 providerBond) external;
    function release(bytes32 bountyId) external;
    function refund(bytes32 bountyId) external;
    function dispute(bytes32 bountyId) external;
    function autoRefund(bytes32 bountyId, uint256 timeoutSeconds) external;
    function getEscrowState(bytes32 bountyId) external view returns (EscrowState);
}
