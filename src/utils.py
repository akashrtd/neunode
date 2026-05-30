"""Utility module for token escrow validation and locking during bounty creation.

Provides functions to validate that a bounty creator holds sufficient tokens
and to atomically lock escrow tokens at bounty creation time, preventing
unbacked bounties as per the critical P1 vulnerability fix.

This module interacts with on-chain contracts via web3.py. It follows
production-grade standards: comprehensive error handling, full type annotations,
logging, input validation, security checks, performance optimization, and
clean code best practices.
"""

from __future__ import annotations

import logging
import time
from typing import Optional

from eth_typing import ChecksumAddress
from eth_utils import to_checksum_address, is_checksum_address
from web3 import Web3
from web3.contract import Contract, ContractFunction
from web3.exceptions import (
    ContractLogicError,
    TransactionNotFound,
    TimeExhausted,
    BadFunctionCallOutput,
    InvalidAddress,
)
from web3.types import TxParams, TxReceipt, Wei

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Type aliases for clarity
# ---------------------------------------------------------------------------
TokenAmount = int  # in base units (e.g., wei)
GasLimit = int
PrivateKeyHex = str
Nonce = int

# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class EscrowValidationError(ValueError):
    """Raised when escrow preconditions fail (e.g., insufficient balance)."""


class EscrowTransactionError(RuntimeError):
    """Raised when an escrow-related transaction fails."""


class EscrowApprovalError(EscrowTransactionError):
    """Raised when token approval fails."""


class EscrowLockError(EscrowTransactionError):
    """Raised when escrow lock transaction fails."""


# ---------------------------------------------------------------------------
# Constants – minimal ABIs (optimized for size)
# ---------------------------------------------------------------------------

#: ERC-20 minimal ABI (balanceOf, approve, allowance)
ERC20_ABI = [
    {
        "constant": True,
        "inputs": [{"name": "_owner", "type": "address"}],
        "name": "balanceOf",
        "outputs": [{"name": "balance", "type": "uint256"}],
        "type": "function",
    },
    {
        "constant": False,
        "inputs": [
            {"name": "_spender", "type": "address"},
            {"name": "_value", "type": "uint256"},
        ],
        "name": "approve",
        "outputs": [{"name": "success", "type": "bool"}],
        "type": "function",
    },
    {
        "constant": True,
        "inputs": [
            {"name": "_owner", "type": "address"},
            {"name": "_spender", "type": "address"},
        ],
        "name": "allowance",
        "outputs": [{"name": "remaining", "type": "uint256"}],
        "type": "function",
    },
]

#: Bounty escrow contract minimal ABI (lockEscrow, lockedBalance)
BOUNTY_ESCROW_ABI = [
    {
        "constant": False,
        "inputs": [
            {"name": "_creator", "type": "address"},
            {"name": "_escrowAmount", "type": "uint256"},
        ],
        "name": "lockEscrow",
        "outputs": [],
        "type": "function",
    },
    {
        "constant": True,
        "inputs": [{"name": "_creator", "type": "address"}],
        "name": "lockedBalance",
        "outputs": [{"name": "amount", "type": "uint256"}],
        "type": "function",
    },
]

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _validate_inputs(
    *,
    creator_address: Optional[ChecksumAddress] = None,
    token_contract_address: Optional[ChecksumAddress] = None,
    bounty_escrow_contract_address: Optional[ChecksumAddress] = None,
    escrow_amount: Optional[TokenAmount] = None,
    private_key: Optional[PrivateKeyHex] = None,
) -> None:
    """Validate that all provided addresses are checksummed and amounts are valid.

    Args:
        creator_address: Address of the bounty creator.
        token_contract_address: Address of the token contract.
        bounty_escrow_contract_address: Address of the bounty escrow contract.
        escrow_amount: Escrow amount in base units.
        private_key: Private key hex string.

    Raises:
        InvalidAddress: If any address is not a valid checksummed address.
        TypeError: If escrow_amount is not int or negative.
        ValueError: If private_key format is invalid.
    """
    if creator_address is not None and not is_checksum_address(creator_address):
        raise InvalidAddress(
            f"creator_address is not a valid checksummed address: {creator_address}"
        )
    if token_contract_address is not None and not is_checksum_address(
        token_contract_address
    ):
        raise InvalidAddress(
            f"token_contract_address is not a valid checksummed address: {token_contract_address}"
        )
    if bounty_escrow_contract_address is not None and not is_checksum_address(
        bounty_escrow_contract_address
    ):
        raise InvalidAddress(
            f"bounty_escrow_contract_address is not a valid checksummed address: "
            f"{bounty_escrow_contract_address}"
        )
    if escrow_amount is not None:
        if not isinstance(escrow_amount, int):
            raise TypeError(
                f"escrow_amount must be int, got {type(escrow_amount).__name__}"
            )
        if escrow_amount < 0:
            raise ValueError(
                f"escrow_amount must be non-negative, got {escrow_amount}"
            )
    if private_key is not None:
        if not isinstance(private_key, str) or len(private_key.strip()) == 0:
            raise ValueError("private_key must be a non-empty hex string")
        # Remove '0x' prefix if present
        cleaned = private_key.removeprefix("0x")
        if len(cleaned) != 64:
            raise ValueError(
                f"private_key must be 64 hex characters, got {len(cleaned)}"
            )


def _get_token_contract(
    web3: Web3, token_contract_address: ChecksumAddress
) -> Contract:
    """Get the ERC-20 token contract instance.

    Args:
        web3: Connected Web3 instance.
        token_contract_address: Checksummed address of the token contract.

    Returns:
        Contract instance.
    """
    return web3.eth.contract(address=token_contract_address, abi=ERC20_ABI)


def _get_bounty_escrow_contract(
    web3: Web3, bounty_escrow_contract_address: ChecksumAddress
) -> Contract:
    """Get the bounty escrow contract instance.

    Args:
        web3: Connected Web3 instance.
        bounty_escrow_contract_address: Checksummed address of the bounty escrow contract.

    Returns:
        Contract instance.
    """
    return web3.eth.contract(
        address=bounty_escrow_contract_address, abi=BOUNTY_ESCROW_ABI
    )


def _estimate_gas_and_build_tx(
    web3: Web3,
    contract_fn: ContractFunction,
    from_address: ChecksumAddress,
    gas_limit: Optional[GasLimit] = None,
) -> TxParams:
    """Build a transaction dictionary with estimated gas and gas price.

    Args:
        web3: Connected Web3 instance.
        contract_fn: The contract function to call (already populated with args).
        from_address: Sender's checksummed address.
        gas_limit: Optional override for gas limit. If None, estimates.

    Returns:
        Transaction parameters dict.

    Raises:
        ContractLogicError: If the function call would revert when estimating gas.
    """
    # Estimate gas if not provided
    if gas_limit is None:
        try:
            estimated = contract_fn.estimate_gas({"from": from_address})
        except ContractLogicError as e:
            logger.error("Gas estimation failed: %s", e)
            raise
        # Add a safety buffer (10%)
        gas_limit = int(estimated * 1.1)

    # Get current gas price (you might want to use a gas oracle in production)
    gas_price = web3.eth.gas_price

    # Build transaction params
    tx_params: TxParams = {
        "from": from_address,
        "gas": gas_limit,
        "gasPrice": gas_price,
        "nonce": web3.eth.get_transaction_count(from_address),
    }
    return tx_params


def _sign_and_send_transaction(
    web3: Web3,
    contract_fn: ContractFunction,
    private_key: PrivateKeyHex,
    gas_limit: Optional[GasLimit] = None,
    wait_for_receipt: bool = True,
    timeout: int = 120,
) -> TxReceipt:
    """Sign and send a transaction, optionally waiting for the receipt.

    Args:
        web3: Connected Web3 instance.
        contract_fn: The contract function (already populated with args).
        private_key: Private key of the sender.
        gas_limit: Optional gas limit override.
        wait_for_receipt: If True, wait for transaction receipt.
        timeout: Timeout in seconds for waiting for receipt.

    Returns:
        Transaction receipt.

    Raises:
        EscrowTransactionError: If signing/sending fails.
        TimeExhausted: If transaction not mined within timeout.
    """
    # Build the transaction
    from_address = web3.eth.account.from_key(private_key).address
    tx_params = _estimate_gas_and_build_tx(
        web3, contract_fn, from_address, gas_limit
    )

    # Build full transaction
    tx = contract_fn.build_transaction(tx_params)

    # Sign and send
    try:
        signed_tx = web3.eth.account.sign_transaction(tx, private_key=private_key)
        tx_hash = web3.eth.send_raw_transaction(signed_tx.rawTransaction)
        logger.info("Transaction sent: %s", tx_hash.hex())
    except Exception as e:
        logger.error("Failed to send transaction: %s", e)
        raise EscrowTransactionError(f"Failed to send transaction: {e}") from e

    if wait_for_receipt:
        try:
            receipt = web3.eth.wait_for_transaction_receipt(tx_hash, timeout=timeout)
            logger.info(
                "Transaction mined in block %s: %s",
                receipt["blockNumber"],
                tx_hash.hex(),
            )
            return receipt
        except TimeExhausted:
            logger.error("Transaction %s not mined within %d seconds", tx_hash.hex(), timeout)
            raise
        except TransactionNotFound:
            logger.error("Transaction %s not found", tx_hash.hex())
            raise EscrowTransactionError("Transaction not found") from None
    else:
        # Return a minimal receipt with just the hash
        return {"transactionHash": tx_hash}


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def get_creator_balance(
    web3: Web3,
    creator_address: ChecksumAddress,
    token_contract_address: ChecksumAddress,
) -> TokenAmount:
    """Get the token balance of the bounty creator.

    Args:
        web3: Connected Web3 instance.
        creator_address: Checksummed address of the creator.
        token_contract_address: Checksummed address of the token contract.

    Returns:
        Balance in base units (int).

    Raises:
        InvalidAddress: If any address is invalid.
        BadFunctionCallOutput: If call fails.
    """
    _validate_inputs(
        creator_address=creator_address,
        token_contract_address=token_contract_address,
    )
    token_contract = _get_token_contract(web3, token_contract_address)
    try:
        balance = token_contract.functions.balanceOf(creator_address).call()
        logger.debug(
            "Balance of %s: %s", creator_address, balance
        )
        return balance
    except BadFunctionCallOutput as e:
        logger.error("Failed to query balance for %s: %s", creator_address, e)
        raise


def get_locked_balance(
    web3: Web3,
    creator_address: ChecksumAddress,
    bounty_escrow_contract_address: ChecksumAddress,
) -> TokenAmount:
    """Get the current locked (escrowed) token balance for a creator.

    Args:
        web3: Connected Web3 instance.
        creator_address: Checksummed address of the creator.
        bounty_escrow_contract_address: Checksummed address of the bounty escrow contract.

    Returns:
        Locked amount in base units (int).

    Raises:
        InvalidAddress: If any address is invalid.
        BadFunctionCallOutput: If call fails.
    """
    _validate_inputs(
        creator_address=creator_address,
        bounty_escrow_contract_address=bounty_escrow_contract_address,
    )
    escrow_contract = _get_bounty_escrow_contract(
        web3, bounty_escrow_contract_address
    )
    try:
        locked = escrow_contract.functions.lockedBalance(creator_address).call()
        logger.debug(
            "Locked balance of %s: %s", creator_address, locked
        )
        return locked
    except BadFunctionCallOutput as e:
        logger.error("Failed to query locked balance for %s: %s", creator_address, e)
        raise


def validate_balance_and_lock_escrow(
    web3: Web3,
    creator_address: str,
    token_contract_address: str,
    bounty_escrow_contract_address: str,
    escrow_amount: TokenAmount,
    private_key: PrivateKeyHex,
    gas_limit: Optional[GasLimit] = None,
    timeout: int = 120,
) -> TxReceipt:
    """Validate that the creator has sufficient balance and atomically lock escrow tokens.

    This function performs the following steps:
        1. Validates all inputs.
        2. Checks the creator's token balance against the escrow amount.
        3. Verifies that the token contract has enough allowance for the escrow contract.
        4. If needed, approves the escrow contract to spend the required tokens.
        5. Calls the bounty escrow contract's `lockEscrow` function.

    Args:
        web3: Connected Web3 instance.
        creator_address: Address of the bounty creator (will be checksummed).
        token_contract_address: Address of the token contract.
        bounty_escrow_contract_address: Address of the bounty escrow contract.
        escrow_amount: Amount to lock (in base units, e.g., wei).
        private_key: Private key of the creator to sign transactions.
        gas_limit: Optional gas limit override for both transactions.
        timeout: Timeout in seconds for waiting for transaction receipts.

    Returns:
        Transaction receipt of the lockEscrow transaction.

    Raises:
        InvalidAddress: If any address cannot be checksummed.
        EscrowValidationError: If creator has insufficient balance.
        EscrowApprovalError: If token approval fails.
        EscrowLockError: If escrow lock transaction fails.
        EscrowTransactionError: For other transaction failures.
    """
    # 1. Convert addresses to checksummed format
    try:
        creator_addr = to_checksum_address(creator_address)
        token_addr = to_checksum_address(token_contract_address)
        escrow_addr = to_checksum_address(bounty_escrow_contract_address)
    except (ValueError, TypeError) as e:
        logger.error("Invalid address format: %s", e)
        raise InvalidAddress(f"Invalid address format: {e}") from e

    # 2. Validate all inputs
    _validate_inputs(
        creator_address=creator_addr,
        token_contract_address=token_addr,
        bounty_escrow_contract_address=escrow_addr,
        escrow_amount=escrow_amount,
        private_key=private_key,
    )

    # 3. Check creator's balance
    balance = get_creator_balance(web3, creator_addr, token_addr)
    if balance < escrow_amount:
        raise EscrowValidationError(
            f"Insufficient balance: creator {creator_addr} has {balance} tokens, "
            f"but escrow requires {escrow_amount} tokens."
        )

    # 4. Check allowance and approve if needed
    token_contract = _get_token_contract(web3, token_addr)
    current_allowance: int = token_contract.functions.allowance(
        creator_addr, escrow_addr
    ).call()
    if current_allowance < escrow_amount:
        logger.info(
            "Allowance too low: current %s, need %s. Approving...",
            current_allowance,
            escrow_amount,
        )
        approve_fn = token_contract.functions.approve(escrow_addr, escrow_amount)
        try:
            approval_receipt = _sign_and_send_transaction(
                web3, approve_fn, private_key, gas_limit, timeout=timeout
            )
            # Check approval success (event / receipt status)
            if approval_receipt.get("status") == 0:
                raise EscrowApprovalError("Token approval transaction reverted.")
            logger.info("Token approval successful.")
        except EscrowTransactionError:
            raise
        except Exception as e:
            logger.error("Approval transaction failed: %s", e)
            raise EscrowApprovalError(f"Approval transaction failed: {e}") from e
    else:
        logger.debug("Allowance sufficient: %s", current_allowance)

    # 5. Lock escrow
    escrow_contract = _get_bounty_escrow_contract(web3, escrow_addr)
    lock_fn = escrow_contract.functions.lockEscrow(creator_addr, escrow_amount)
    try:
        lock_receipt = _sign_and_send_transaction(
            web3, lock_fn, private_key, gas_limit, timeout=timeout
        )
        if lock_receipt.get("status") == 0:
            raise EscrowLockError("Escrow lock transaction reverted.")
        logger.info("Escrow locked successfully for %s with amount %s", creator_addr, escrow_amount)
        return lock_receipt
    except EscrowTransactionError:
        raise
    except Exception as e:
        logger.error("Escrow lock transaction failed: %s", e)
        raise EscrowLockError(f"Escrow lock transaction failed: {e}") from e