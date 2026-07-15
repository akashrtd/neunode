// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.24;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {Math} from "@openzeppelin/contracts/utils/math/Math.sol";

/// @title ResourceAMM — Treasury-seeded exchange for Neunode resource tokens
/// @notice Maintains one constant-product pool for each configured token pair.
///         Pools are protocol-owned: only treasury governance can seed/add liquidity,
///         while any agent may swap with explicit slippage and deadline protection.
contract ResourceAMM is AccessControl, ReentrancyGuard {
    using SafeERC20 for IERC20;

    struct Pool {
        uint256 reserve0;
        uint256 reserve1;
    }

    bytes32 public constant TREASURY_ROLE = keccak256("TREASURY_ROLE");
    uint256 public constant BPS_DENOMINATOR = 10_000;
    uint256 public constant SWAP_FEE_BPS = 30;

    mapping(address => bool) public allowedTokens;
    mapping(bytes32 => Pool) private _pools;

    event PoolSeeded(
        address indexed token0, address indexed token1, uint256 amount0, uint256 amount1
    );
    event LiquidityAdded(
        address indexed token0, address indexed token1, uint256 amount0, uint256 amount1
    );
    event Swap(
        address indexed sender,
        address indexed tokenIn,
        address indexed tokenOut,
        uint256 amountIn,
        uint256 amountOut,
        address recipient
    );

    error InvalidToken();
    error IdenticalTokens();
    error ZeroAmount();
    error PoolAlreadyInitialized();
    error PoolNotInitialized();
    error DeadlineExpired(uint256 deadline, uint256 timestamp);
    error InsufficientOutput(uint256 minimum, uint256 actual);
    error InsufficientLiquidity();
    error InvalidRecipient();

    constructor(address[4] memory tokens, address treasury) {
        if (treasury == address(0)) revert InvalidRecipient();
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(TREASURY_ROLE, treasury);
        for (uint256 i = 0; i < tokens.length; i++) {
            if (tokens[i] == address(0)) revert InvalidToken();
            for (uint256 j = 0; j < i; j++) {
                if (tokens[i] == tokens[j]) revert IdenticalTokens();
            }
            allowedTokens[tokens[i]] = true;
        }
    }

    /// @notice Initialize a pair with protocol-owned treasury liquidity.
    function seedPool(address tokenA, address tokenB, uint256 amountA, uint256 amountB)
        external
        onlyRole(TREASURY_ROLE)
        nonReentrant
    {
        (address token0, address token1, bool reversed) = _pair(tokenA, tokenB);
        if (amountA == 0 || amountB == 0) revert ZeroAmount();
        Pool storage pool = _pools[_pairKey(token0, token1)];
        if (pool.reserve0 != 0 || pool.reserve1 != 0) revert PoolAlreadyInitialized();

        (uint256 amount0, uint256 amount1) = reversed ? (amountB, amountA) : (amountA, amountB);
        IERC20(token0).safeTransferFrom(msg.sender, address(this), amount0);
        IERC20(token1).safeTransferFrom(msg.sender, address(this), amount1);
        pool.reserve0 = amount0;
        pool.reserve1 = amount1;
        emit PoolSeeded(token0, token1, amount0, amount1);
    }

    /// @notice Add protocol liquidity in the current reserve ratio.
    function addLiquidity(address tokenA, address tokenB, uint256 amountA, uint256 amountB)
        external
        onlyRole(TREASURY_ROLE)
        nonReentrant
    {
        (address token0, address token1, bool reversed) = _pair(tokenA, tokenB);
        if (amountA == 0 || amountB == 0) revert ZeroAmount();
        Pool storage pool = _pools[_pairKey(token0, token1)];
        if (pool.reserve0 == 0 || pool.reserve1 == 0) revert PoolNotInitialized();
        (uint256 amount0, uint256 amount1) = reversed ? (amountB, amountA) : (amountA, amountB);
        if (Math.mulDiv(amount0, pool.reserve1, pool.reserve0) != amount1) {
            revert InsufficientLiquidity();
        }

        IERC20(token0).safeTransferFrom(msg.sender, address(this), amount0);
        IERC20(token1).safeTransferFrom(msg.sender, address(this), amount1);
        pool.reserve0 += amount0;
        pool.reserve1 += amount1;
        emit LiquidityAdded(token0, token1, amount0, amount1);
    }

    /// @notice Quote an exact-input swap using the current reserves and 30 bps fee.
    function quoteExactInput(address tokenIn, address tokenOut, uint256 amountIn)
        public
        view
        returns (uint256 amountOut)
    {
        (uint256 reserveIn, uint256 reserveOut) = _directedReserves(tokenIn, tokenOut);
        if (amountIn == 0) revert ZeroAmount();
        uint256 amountInWithFee = amountIn * (BPS_DENOMINATOR - SWAP_FEE_BPS);
        amountOut =
            Math.mulDiv(amountInWithFee, reserveOut, reserveIn * BPS_DENOMINATOR + amountInWithFee);
        if (amountOut == 0 || amountOut >= reserveOut) revert InsufficientLiquidity();
    }

    /// @notice Swap an exact input amount for at least `minimumOut` before `deadline`.
    function swapExactInput(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        uint256 minimumOut,
        address recipient,
        uint256 deadline
    ) external nonReentrant returns (uint256 amountOut) {
        if (block.timestamp > deadline) {
            revert DeadlineExpired(deadline, block.timestamp);
        }
        if (recipient == address(0)) revert InvalidRecipient();
        amountOut = quoteExactInput(tokenIn, tokenOut, amountIn);
        if (amountOut < minimumOut) revert InsufficientOutput(minimumOut, amountOut);

        (address token0, address token1, bool reversed) = _pair(tokenIn, tokenOut);
        Pool storage pool = _pools[_pairKey(token0, token1)];
        if (reversed) {
            pool.reserve1 += amountIn;
            pool.reserve0 -= amountOut;
        } else {
            pool.reserve0 += amountIn;
            pool.reserve1 -= amountOut;
        }

        IERC20(tokenIn).safeTransferFrom(msg.sender, address(this), amountIn);
        IERC20(tokenOut).safeTransfer(recipient, amountOut);
        emit Swap(msg.sender, tokenIn, tokenOut, amountIn, amountOut, recipient);
    }

    function getReserves(address tokenA, address tokenB)
        external
        view
        returns (uint256 reserveA, uint256 reserveB)
    {
        (address token0, address token1, bool reversed) = _pair(tokenA, tokenB);
        Pool storage pool = _pools[_pairKey(token0, token1)];
        return reversed ? (pool.reserve1, pool.reserve0) : (pool.reserve0, pool.reserve1);
    }

    function _directedReserves(address tokenIn, address tokenOut)
        internal
        view
        returns (uint256 reserveIn, uint256 reserveOut)
    {
        (address token0, address token1, bool reversed) = _pair(tokenIn, tokenOut);
        Pool storage pool = _pools[_pairKey(token0, token1)];
        if (pool.reserve0 == 0 || pool.reserve1 == 0) revert PoolNotInitialized();
        return reversed ? (pool.reserve1, pool.reserve0) : (pool.reserve0, pool.reserve1);
    }

    function _pair(address tokenA, address tokenB)
        internal
        view
        returns (address token0, address token1, bool reversed)
    {
        if (tokenA == tokenB) revert IdenticalTokens();
        if (!allowedTokens[tokenA] || !allowedTokens[tokenB]) revert InvalidToken();
        reversed = tokenA > tokenB;
        (token0, token1) = reversed ? (tokenB, tokenA) : (tokenA, tokenB);
    }

    function _pairKey(address token0, address token1) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(token0, token1));
    }
}
