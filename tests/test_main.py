"""Tests for token escrow validation at bounty creation.

This test module verifies that bounty creation validates token balance
and locks escrow tokens as specified in P1 security issue:
"Token escrow not validated at bounty creation" (v2 technical gaps audit).

Features tested:
- Bounty creation with zero balance is rejected
- Bounty creation with sufficient balance succeeds
- Escrow tokens are deducted from creator's available balance
- Escrowed balance is tracked separately if supported
- API response validation and error detail checks
"""

import logging
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, Generator, Optional

import httpx
import pytest

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

API_BASE_URL: str = "http://localhost:8080/api/v1"
"""Default base URL for the API under test."""

DEFAULT_TIMEOUT: float = 30.0
"""HTTP request timeout in seconds."""

ZERO_BALANCE: str = "0"
"""Initial token balance for identity with no funds."""

SUFFICIENT_BALANCE: str = "1000"
"""Initial token balance for identity with enough funds."""

BOUNTY_ESCROW_AMOUNT: str = "500"
"""Escrow amount used in successful bounty creation tests."""

INSUFFICIENT_ESCROW_AMOUNT: str = "1000"
"""Escrow amount exceeding zero balance, used to test rejection."""

BOUNTY_DEADLINE: str = "2027-06-01T00:00:00Z"
"""Standard deadline for test bounties."""

LOCKED_BALANCE_FIELD: str = "locked_balance"
"""Field name for locked token balance if separate tracking is enabled."""

# ---------------------------------------------------------------------------
# Logging configuration
# ---------------------------------------------------------------------------

logger = logging.getLogger(__name__)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S",
)

# ---------------------------------------------------------------------------
# Custom exceptions
# ---------------------------------------------------------------------------


class TestVerificationError(AssertionError):
    """Raised when a test assertion or verification fails."""


class APIClientError(RuntimeError):
    """Raised on unexpected API client behaviour."""


# ---------------------------------------------------------------------------
# Data representation for API responses
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class IdentityResponse:
    """Typed representation of a created identity."""

    id: str
    available_balance: str
    locked_balance: str = "0"


@dataclass(frozen=True)
class BountyResponse:
    """Typed representation of a created bounty."""

    id: str
    creator_id: str
    title: str
    escrow_amount: str
    deadline: str


# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------


def _parse_identity_response(response_data: Dict[str, Any]) -> IdentityResponse:
    """Parse and validate an identity creation response.

    Args:
        response_data: Parsed JSON response from identity creation endpoint.

    Returns:
        IdentityResponse with validated fields.

    Raises:
        TestVerificationError: If the response lacks required fields or has invalid types.
        InvalidOperation: If balance fields are not valid decimals.
    """
    required_fields = {"id", "available_balance"}
    missing = required_fields - response_data.keys()
    if missing:
        raise TestVerificationError(
            f"Identity response missing required fields: {missing}"
        )

    identity_id = response_data.get("id")
    if not isinstance(identity_id, str) or not identity_id.strip():
        raise TestVerificationError(
            f"Identity 'id' must be a non-empty string, got {identity_id!r}"
        )

    available_balance = response_data.get("available_balance")
    if not isinstance(available_balance, str) or not available_balance.strip():
        raise TestVerificationError(
            f"Identity 'available_balance' must be a non-empty string, got {available_balance!r}"
        )
    Decimal(available_balance)  # validate numeric

    locked_balance = response_data.get(LOCKED_BALANCE_FIELD, "0")
    if not isinstance(locked_balance, str):
        raise TestVerificationError(
            f"Identity '{LOCKED_BALANCE_FIELD}' must be a string, got {type(locked_balance).__name__}"
        )
    Decimal(locked_balance)  # validate numeric

    return IdentityResponse(
        id=identity_id,
        available_balance=available_balance,
        locked_balance=locked_balance,
    )


def _parse_bounty_response(response_data: Dict[str, Any]) -> BountyResponse:
    """Parse and validate a bounty creation response.

    Args:
        response_data: Parsed JSON response from bounty creation endpoint.

    Returns:
        BountyResponse with validated fields.

    Raises:
        TestVerificationError: If required fields are missing or types are wrong.
        InvalidOperation: If numeric fields are not valid decimals.
    """
    required_fields = {"id", "creator_id", "title", "escrow_amount", "deadline"}
    missing = required_fields - response_data.keys()
    if missing:
        raise TestVerificationError(
            f"Bounty response missing required fields: {missing}"
        )

    for field in ("id", "creator_id", "title", "escrow_amount", "deadline"):
        value = response_data.get(field)
        if not isinstance(value, str) or not value.strip():
            raise TestVerificationError(
                f"Bounty '{field}' must be a non-empty string, got {value!r}"
            )

    # Validate numeric fields
    Decimal(response_data["escrow_amount"])

    return BountyResponse(
        id=response_data["id"],
        creator_id=response_data["creator_id"],
        title=response_data["title"],
        escrow_amount=response_data["escrow_amount"],
        deadline=response_data["deadline"],
    )


def _safe_delete(
    client: httpx.Client,
    method: str,
    url: str,
    resource_id: str,
    expected_status: Optional[int] = None,
) -> None:
    """Delete or clean up a resource by ID, logging any failures.

    Args:
        client: HTTP client instance.
        method: HTTP method for cleanup (e.g., ``"DELETE"``).
        url: Resource base URL (without ID), e.g., ``"/identities"``.
        resource_id: Unique identifier of the resource.
        expected_status: If provided, log a warning if the response status
            does not match (e.g., 204 for delete success).
    """
    if not resource_id:
        return
    full_url = f"{url}/{resource_id}"
    try:
        response = client.request(method, full_url)
        if expected_status is not None:
            if response.status_code != expected_status:
                logger.warning(
                    "Cleanup of %s returned status %d (expected %d): %s",
                    full_url,
                    response.status_code,
                    expected_status,
                    response.text[:300],
                )
        else:
            response.raise_for_status()
        logger.info("Cleaned up resource %s", full_url)
    except httpx.HTTPStatusError as exc:
        logger.warning(
            "Cleanup of %s failed with status %d: %s",
            full_url,
            exc.response.status_code,
            exc.response.text[:300],
        )
    except httpx.RequestError as exc:
        logger.error(
            "Network error during cleanup of %s: %s",
            full_url,
            exc,
        )


def _delete_identity(client: httpx.Client, identity_id: str) -> None:
    """Convenience wrapper to delete an identity by ID.

    Args:
        client: HTTP client instance.
        identity_id: The identity's unique identifier.
    """
    _safe_delete(
        client, "DELETE", f"{API_BASE_URL}/identities", identity_id, expected_status=204
    )


def _delete_bounty(client: httpx.Client, bounty_id: str) -> None:
    """Convenience wrapper to delete a bounty by ID.

    Args:
        client: HTTP client instance.
        bounty_id: The bounty's unique identifier.
    """
    _safe_delete(
        client, "DELETE", f"{API_BASE_URL}/bounties", bounty_id, expected_status=204
    )


def create_identity(
    client: httpx.Client, initial_balance: str, label: str = "test-identity"
) -> IdentityResponse:
    """Create an identity with the given token balance.

    Args:
        client: HTTP client instance.
        initial_balance: Token balance to set for the new identity (string decimal).
        label: Optional human-readable label for the identity.

    Returns:
        IdentityResponse with the created identity's data.

    Raises:
        APIClientError: If the API returns an unexpected status code.
        TestVerificationError: If response validation fails.
    """
    payload: Dict[str, Any] = {
        "label": label,
        "initial_balance": initial_balance,
    }
    try:
        response = client.post(
            f"{API_BASE_URL}/identities",
            json=payload,
            timeout=DEFAULT_TIMEOUT,
        )
    except httpx.TimeoutException as exc:
        raise APIClientError("Identity creation request timed out") from exc
    except httpx.RequestError as exc:
        raise APIClientError(f"Identity creation request failed: {exc}") from exc

    if response.status_code != 201:
        raise APIClientError(
            f"Identity creation failed with status {response.status_code}: "
            f"{response.text[:300]}"
        )

    data = response.json()
    return _parse_identity_response(data)


def create_bounty(
    client: httpx.Client,
    creator_id: str,
    title: str,
    escrow_amount: str,
    deadline: str,
) -> BountyResponse:
    """Create a bounty for the given creator.

    Args:
        client: HTTP client instance.
        creator_id: The unique ID of the bounty creator.
        title: Bounty title.
        escrow_amount: Amount of tokens to escrow (string decimal).
        deadline: ISO 8601 date-time string for the bounty deadline.

    Returns:
        BountyResponse with the created bounty's data.

    Raises:
        APIClientError: If the API returns an unexpected status code (including 400 for
            validation errors).
        TestVerificationError: If response validation fails.
    """
    payload: Dict[str, Any] = {
        "creator_id": creator_id,
        "title": title,
        "escrow_amount": escrow_amount,
        "deadline": deadline,
    }
    try:
        response = client.post(
            f"{API_BASE_URL}/bounties",
            json=payload,
            timeout=DEFAULT_TIMEOUT,
        )
    except httpx.TimeoutException as exc:
        raise APIClientError("Bounty creation request timed out") from exc
    except httpx.RequestError as exc:
        raise APIClientError(f"Bounty creation request failed: {exc}") from exc

    if response.status_code == 400:
        # Validation error – expected in some tests, we let the caller handle
        # but still raise a structured exception to support clean test flow.
        raise APIClientError(
            f"Bounty creation rejected with status 400: {response.text[:300]}"
        ) from None
    if response.status_code != 201:
        raise APIClientError(
            f"Bounty creation failed with status {response.status_code}: "
            f"{response.text[:300]}"
        )

    data = response.json()
    return _parse_bounty_response(data)


def get_identity_balance(
    client: httpx.Client, identity_id: str
) -> IdentityResponse:
    """Fetch the current balance (available and locked) for an identity.

    Args:
        client: HTTP client instance.
        identity_id: The identity's unique ID.

    Returns:
        IdentityResponse with updated balance fields.

    Raises:
        APIClientError: If the API request fails.
    """
    try:
        response = client.get(
            f"{API_BASE_URL}/identities/{identity_id}",
            timeout=DEFAULT_TIMEOUT,
        )
    except httpx.TimeoutException as exc:
        raise APIClientError("Balance fetch request timed out") from exc
    except httpx.RequestError as exc:
        raise APIClientError(f"Balance fetch request failed: {exc}") from exc

    if response.status_code != 200:
        raise APIClientError(
            f"Balance fetch failed with status {response.status_code}: "
            f"{response.text[:300]}"
        )

    data = response.json()
    return _parse_identity_response(data)


def assert_escrow_deducted(
    client: httpx.Client,
    identity_id: str,
    original_balance: str,
    escrow_amount: str,
    separate_locked_tracking: bool = True,
) -> None:
    """Verify that escrow amount is properly reflected in the creator's balance.

    Checks that available_balance decreased by escrow_amount, and if separate
    locked tracking is supported, that locked_balance increased by escrow_amount.

    Args:
        client: HTTP client instance.
        identity_id: The identity's unique ID.
        original_balance: The available balance before bounty creation.
        escrow_amount: The amount that was escrowed.
        separate_locked_tracking: If True, also validate locked_balance field.

    Raises:
        TestVerificationError: If balance after escrow is inconsistent.
    """
    current = get_identity_balance(client, identity_id)
    original = Decimal(original_balance)
    escrow = Decimal(escrow_amount)
    expected_available = original - escrow

    actual_available = Decimal(current.available_balance)
    if actual_available != expected_available:
        raise TestVerificationError(
            f"Available balance mismatch: expected {expected_available}, "
            f"got {actual_available}"
        )

    if separate_locked_tracking:
        expected_locked = escrow
        actual_locked = Decimal(current.locked_balance)
        if actual_locked != expected_locked:
            raise TestVerificationError(
                f"Locked balance mismatch: expected {expected_locked}, "
                f"got {actual_locked}"
            )


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="function")
def client() -> Generator[httpx.Client, None, None]:
    """Provide an HTTP client with base URL and default timeout.

    Yields:
        httpx.Client instance configured for the test API.
    """
    headers = {"Content-Type": "application/json", "Accept": "application/json"}
    with httpx.Client(base_url=API_BASE_URL, timeout=DEFAULT_TIMEOUT, headers=headers) as session:
        yield session


@pytest.fixture
def identity_with_zero_balance(client: httpx.Client) -> Generator[IdentityResponse, None, None]:
    """Create an identity with zero tokens and clean up after test.

    Yields:
        IdentityResponse for the created identity.
    """
    identity = create_identity(client, ZERO_BALANCE, label="zero-balance-test")
    yield identity
    _delete_identity(client, identity.id)


@pytest.fixture
def identity_with_sufficient_balance(client: httpx.Client) -> Generator[IdentityResponse, None, None]:
    """Create an identity with sufficient tokens for escrow and clean up after test.

    Yields:
        IdentityResponse for the created identity.
    """
    identity = create_identity(client, SUFFICIENT_BALANCE, label="sufficient-balance-test")
    yield identity
    _delete_identity(client, identity.id)


# ---------------------------------------------------------------------------
# Test cases
# ---------------------------------------------------------------------------


def test_create_bounty_with_zero_balance_rejected(
    client: httpx.Client,
    identity_with_zero_balance: IdentityResponse,
) -> None:
    """Test that bounty creation fails when the creator has zero token balance.

    Steps:
        1. Create identity with 0 tokens.
        2. Attempt to create a bounty with escrow amount > 0.
        3. Assert that creation is rejected with a 4xx status (insufficient balance).
        4. Verify error message mentions insufficient balance.

    Args:
        client: HTTP client fixture.
        identity_with_zero_balance: Identity with zero balance.
    """
    creator = identity_with_zero_balance
    with pytest.raises(APIClientError) as exc_info:
        create_bounty(
            client,
            creator_id=creator.id,
            title="Test bounty with no funds",
            escrow_amount=INSUFFICIENT_ESCROW_AMOUNT,
            deadline=BOUNTY_DEADLINE,
        )
    # Verify the error message is appropriate
    error_detail = str(exc_info.value)
    assert "400" in error_detail, (
        f"Expected a 400 status error for insufficient balance, got: {error_detail}"
    )


def test_create_bounty_with_sufficient_balance_succeeds(
    client: httpx.Client,
    identity_with_sufficient_balance: IdentityResponse,
) -> None:
    """Test that bounty creation succeeds when the creator has enough tokens.

    Also validates that the escrow tokens are locked (available balance reduced).

    Steps:
        1. Create identity with 1000 tokens.
        2. Capture current available_balance.
        3. Create bounty with escrow amount 500.
        4. Assert creation returns 201 and a valid BountyResponse.
        5. Fetch identity again; verify available_balance decreased by 500.
        6. If locked_balance is supported, verify it increased by 500.

    Args:
        client: HTTP client fixture.
        identity_with_sufficient_balance: Identity with sufficient tokens.
    """
    creator = identity_with_sufficient_balance
    original_balance = creator.available_balance

    bounty = create_bounty(
        client,
        creator_id=creator.id,
        title="Test bounty with sufficient funds",
        escrow_amount=BOUNTY_ESCROW_AMOUNT,
        deadline=BOUNTY_DEADLINE,
    )
    # Basic validation of returned bounty
    assert bounty.creator_id == creator.id
    assert bounty.escrow_amount == BOUNTY_ESCROW_AMOUNT

    # Verify escrow deduction
    assert_escrow_deducted(
        client,
        identity_id=creator.id,
        original_balance=original_balance,
        escrow_amount=BOUNTY_ESCROW_AMOUNT,
        separate_locked_tracking=True,
    )

    # Cleanup bounty after test
    _delete_bounty(client, bounty.id)


def test_escrow_locks_tokens_atomic(
    client: httpx.Client,
    identity_with_sufficient_balance: IdentityResponse,
) -> None:
    """Test that escrow tokens are atomically locked at bounty creation.

    Verifies that the available balance is reduced immediately and locked
    balance is reflected in the same API response round.

    Steps:
        1. Create identity with sufficient balance.
        2. Record initial available and locked balances.
        3. Create bounty with half of the balance as escrow.
        4. Immediately fetch updated balance without further transactions.
        5. Assert available decreased by exactly the escrow amount.
        6. Assert locked increased by the same amount (if supported).

    Args:
        client: HTTP client fixture.
        identity_with_sufficient_balance: Identity with sufficient tokens.
    """
    creator = identity_with_sufficient_balance
    original_balance = creator.available_balance

    # Create bounty
    bounty = create_bounty(
        client,
        creator_id=creator.id,
        title="Atomic lock test",
        escrow_amount=BOUNTY_ESCROW_AMOUNT,
        deadline=BOUNTY_DEADLINE,
    )

    # Immediately check balance (no other operations)
    updated = get_identity_balance(client, creator.id)
    expected_available = Decimal(original_balance) - Decimal(BOUNTY_ESCROW_AMOUNT)
    actual_available = Decimal(updated.available_balance)
    assert actual_available == expected_available, (
        f"Available balance not atomically reduced: "
        f"expected {expected_available}, got {actual_available}"
    )

    if LOCKED_BALANCE_FIELD is not None:
        expected_locked = Decimal(BOUNTY_ESCROW_AMOUNT)
        actual_locked = Decimal(updated.locked_balance)
        assert actual_locked == expected_locked, (
            f"Locked balance not atomically increased: "
            f"expected {expected_locked}, got {actual_locked}"
        )

    # Cleanup
    _delete_bounty(client, bounty.id)


def test_create_bounty_escrow_exceeds_sufficient_balance(
    client: httpx.Client,
    identity_with_sufficient_balance: IdentityResponse,
) -> None:
    """Test that creation fails when escrow amount > creator's balance.

    Even with a non-zero balance, if escrow exceeds it, the API must reject.

    Steps:
        1. Create identity with 1000 tokens.
        2. Attempt to create bounty with escrow amount 1500.
        3. Assert rejection with 4xx status and appropriate error.
        4. Verify balance remains unchanged.

    Args:
        client: HTTP client fixture.
        identity_with_sufficient_balance: Identity with 1000 tokens.
    """
    creator = identity_with_sufficient_balance
    original_balance = creator.available_balance

    with pytest.raises(APIClientError) as exc_info:
        create_bounty(
            client,
            creator_id=creator.id,
            title="Bounty with excessive escrow",
            escrow_amount="1500",  # exceeds 1000
            deadline=BOUNTY_DEADLINE,
        )
    assert "400" in str(exc_info.value), (
        f"Expected a 400 error for exceeding balance, got: {exc_info.value}"
    )

    # Verify balance unchanged
    current = get_identity_balance(client, creator.id)
    assert current.available_balance == original_balance, (
        f"Balance changed despite rejected creation: "
        f"original {original_balance}, current {current.available_balance}"
    )
    if LOCKED_BALANCE_FIELD is not None:
        assert Decimal(current.locked_balance) == 0, (
            f"Locked balance should remain zero after rejected creation, "
            f"got {current.locked_balance}"
        )


def test_multiple_bounties_multiple_escrows(
    client: httpx.Client,
    identity_with_sufficient_balance: IdentityResponse,
) -> None:
    """Test that multiple bounties correctly accumulate locked escrow.

    Steps:
        1. Create identity with 1000 tokens.
        2. Create two bounties each with escrow 300 (total 600).
        3. Verify available balance decreased by 600.
        4. Verify locked balance increased by 600 (if supported).
        5. Verify third bounty with remaining 400 succeeds.
        6. Verify total locked = 1000 and available = 0.

    Args:
        client: HTTP client fixture.
        identity_with_sufficient_balance: Identity with 1000 tokens.
    """
    creator = identity_with_sufficient_balance
    original_balance = Decimal(creator.available_balance)

    # First bounty
    bounty1 = create_bounty(
        client,
        creator_id=creator.id,
        title="First multiple escrow",
        escrow_amount="300",
        deadline=BOUNTY_DEADLINE,
    )
    # Second bounty
    bounty2 = create_bounty(
        client,
        creator_id=creator.id,
        title="Second multiple escrow",
        escrow_amount="300",
        deadline=BOUNTY_DEADLINE,
    )
    # Third bounty with remaining balance
    bounty3 = create_bounty(
        client,
        creator_id=creator.id,
        title="Third and last escrow",
        escrow_amount="400",
        deadline=BOUNTY_DEADLINE,
    )

    current = get_identity_balance(client, creator.id)
    expected_available = Decimal(0)
    assert Decimal(current.available_balance) == expected_available, (
        f"Expected available balance 0 after full escrow, got {current.available_balance}"
    )
    if LOCKED_BALANCE_FIELD is not None:
        expected_locked = Decimal("1000")
        assert Decimal(current.locked_balance) == expected_locked, (
            f"Expected locked balance {expected_locked}, got {current.locked_balance}"
        )

    # Cleanup
    for bounty in (bounty3, bounty2, bounty1):
        _delete_bounty(client, bounty.id)


def test_api_error_message_for_insufficient_balance(
    client: httpx.Client,
    identity_with_zero_balance: IdentityResponse,
) -> None:
    """Test that the API returns a clear error message when balance is insufficient.

    The error message should indicate "insufficient balance" or similar.

    Steps:
        1. Create identity with 0 tokens.
        2. Attempt to create bounty.
        3. Check that error detail contains expected wording.
    """
    creator = identity_with_zero_balance
    expected_phrases = [
        "insufficient",
        "balance",
        "escrow",
        "funds",
    ]

    try:
        create_bounty(
            client,
            creator_id=creator.id,
            title="Test error message",
            escrow_amount=INSUFFICIENT_ESCROW_AMOUNT,
            deadline=BOUNTY_DEADLINE,
        )
        pytest.fail("Expected APIClientError was not raised")
    except APIClientError as exc:
        error_text = str(exc.value).lower()
        assert any(phrase in error_text for phrase in expected_phrases), (
            f"Error message does not contain any of the expected phrases "
            f"{expected_phrases}. Full error: {exc.value}"
        )