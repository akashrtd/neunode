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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, FixedBytes, U256};
    use alloy::primitives::{address, fixed_bytes};
    use alloy::sol_types::{SolError, SolEvent};

    // ─── EscrowState enum tests ─────────────────────────────────────────────

    #[test]
    fn escrow_state_all_variants() {
        let _created = EscrowState::Created;
        let _funded = EscrowState::Funded;
        let _completed = EscrowState::Completed;
        let _refunded = EscrowState::Refunded;
        let _disputed = EscrowState::Disputed;
    }

    #[test]
    fn escrow_state_equality() {
        assert_eq!(EscrowState::Created, EscrowState::Created);
        assert_ne!(EscrowState::Created, EscrowState::Funded);
        assert_ne!(EscrowState::Completed, EscrowState::Refunded);
        assert_ne!(EscrowState::Disputed, EscrowState::Created);
    }

    // ─── Escrow struct tests ────────────────────────────────────────────────

    #[test]
    fn escrow_construction() {
        let escrow = Escrow {
            bountyId: fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001"),
            requester: address!("0000000000000000000000000000000000000001"),
            provider: address!("0000000000000000000000000000000000000002"),
            token: address!("0000000000000000000000000000000000000003"),
            amount: U256::from(5000),
            providerBond: U256::from(750),
            created: U256::from(100),
            deadline: U256::from(1000),
            state: EscrowState::Created,
        };
        assert_eq!(escrow.state, EscrowState::Created);
        assert_eq!(escrow.amount, U256::from(5000));
        assert_eq!(escrow.providerBond, U256::from(750));
        assert_eq!(escrow.bountyId, fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001"));
    }

    #[test]
    fn escrow_provider_bond_is_15_pct() {
        let amount = U256::from(10000);
        let bond = amount * U256::from(15) / U256::from(100);
        assert_eq!(bond, U256::from(1500));
    }

    #[test]
    fn escrow_zero_fields() {
        let escrow = Escrow {
            bountyId: FixedBytes::<32>::ZERO,
            requester: Address::ZERO,
            provider: Address::ZERO,
            token: Address::ZERO,
            amount: U256::ZERO,
            providerBond: U256::ZERO,
            created: U256::ZERO,
            deadline: U256::ZERO,
            state: EscrowState::Created,
        };
        assert_eq!(escrow.amount, U256::ZERO);
        assert_eq!(escrow.provider, Address::ZERO);
    }

    // ─── Event signature tests ──────────────────────────────────────────────

    #[test]
    fn event_signatures_non_empty() {
        assert!(!EscrowCreated::SIGNATURE.is_empty());
        assert!(!EscrowFunded::SIGNATURE.is_empty());
        assert!(!EscrowReleased::SIGNATURE.is_empty());
        assert!(!EscrowRefunded::SIGNATURE.is_empty());
        assert!(!EscrowDisputed::SIGNATURE.is_empty());
        assert!(!EscrowReleasedWithFees::SIGNATURE.is_empty());
        assert!(!BountyContractRegistered::SIGNATURE.is_empty());
    }

    #[test]
    fn event_signatures_expected_format() {
        assert!(EscrowCreated::SIGNATURE.starts_with("EscrowCreated("));
        assert!(EscrowFunded::SIGNATURE.starts_with("EscrowFunded("));
        assert!(EscrowReleased::SIGNATURE.starts_with("EscrowReleased("));
        assert!(EscrowRefunded::SIGNATURE.starts_with("EscrowRefunded("));
        assert!(EscrowDisputed::SIGNATURE.starts_with("EscrowDisputed("));
    }

    #[test]
    fn event_selectors_are_32_bytes() {
        assert_eq!(EscrowCreated::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(EscrowFunded::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(EscrowReleased::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(EscrowRefunded::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(EscrowDisputed::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(EscrowReleasedWithFees::SIGNATURE_HASH.as_slice().len(), 32);
        assert_eq!(BountyContractRegistered::SIGNATURE_HASH.as_slice().len(), 32);
    }

    #[test]
    fn event_selectors_unique() {
        let selectors = [
            EscrowCreated::SIGNATURE_HASH,
            EscrowFunded::SIGNATURE_HASH,
            EscrowReleased::SIGNATURE_HASH,
            EscrowRefunded::SIGNATURE_HASH,
            EscrowDisputed::SIGNATURE_HASH,
            EscrowReleasedWithFees::SIGNATURE_HASH,
            BountyContractRegistered::SIGNATURE_HASH,
        ];
        for i in 0..selectors.len() {
            for j in (i + 1)..selectors.len() {
                assert_ne!(
                    selectors[i], selectors[j],
                    "Escrow event selectors must be unique"
                );
            }
        }
    }

    // ─── Error construction tests ───────────────────────────────────────────

    #[test]
    fn error_types_constructible() {
        let bounty_id = fixed_bytes!("0000000000000000000000000000000000000000000000000000000000000001");
        let caller = address!("0000000000000000000000000000000000000001");

        let _ = EscrowNotFound { bountyId: bounty_id };
        let _ = EscrowAlreadyExists { bountyId: bounty_id };
        let _ = EscrowNotCreated { bountyId: bounty_id };
        let _ = EscrowNotFunded { bountyId: bounty_id };
        let _ = NotRequester {
            bountyId: bounty_id,
            caller,
        };
        let _ = NotProvider {
            bountyId: bounty_id,
            caller,
        };
        let _ = InvalidAmount {};
        let _ = InvalidToken {};
        let _ = DeadlinePassed {
            deadline: U256::from(100),
        };
        let _ = Unauthorized {};
        let _ = FeeBpsExceeds100Pct {
            totalBps: U256::from(10100),
        };
        let _ = ZeroAddressFeeRecipient {};
    }

    #[test]
    fn error_selectors_are_4_bytes() {
        assert_eq!(EscrowNotFound::SELECTOR.len(), 4);
        assert_eq!(EscrowAlreadyExists::SELECTOR.len(), 4);
        assert_eq!(EscrowNotCreated::SELECTOR.len(), 4);
        assert_eq!(EscrowNotFunded::SELECTOR.len(), 4);
        assert_eq!(NotRequester::SELECTOR.len(), 4);
        assert_eq!(NotProvider::SELECTOR.len(), 4);
        assert_eq!(InvalidAmount::SELECTOR.len(), 4);
        assert_eq!(InvalidToken::SELECTOR.len(), 4);
        assert_eq!(DeadlinePassed::SELECTOR.len(), 4);
        assert_eq!(Unauthorized::SELECTOR.len(), 4);
        assert_eq!(FeeBpsExceeds100Pct::SELECTOR.len(), 4);
        assert_eq!(ZeroAddressFeeRecipient::SELECTOR.len(), 4);
    }
}
