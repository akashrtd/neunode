// SPDX-License-Identifier: MIT
pragma solidity 0.8.18;

/**
 * @title NeunodeToken
 * @notice ERC‑20 token with resource‑backed minting and configurable balance decay.
 * @dev Decay reduces each holder's balance proportionally over time. The decay
 *      is applied on every state‑changing operation for the sender. Total supply
 *      shrinks by the decayed amount. Minting is restricted to a designated minter.
 */
contract NeunodeToken {
    // ============ ERRORS ============
    error NeunodeToken__Unauthorized();
    error NeunodeToken__InvalidDecayRate();
    error NeunodeToken__InsufficientBalance();
    error NeunodeToken__InsufficientAllowance();
    error NeunodeToken__ZeroAddress();

    // ============ EVENTS ============
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event MinterChanged(address indexed previousMinter, address indexed newMinter);
    event AdminChanged(address indexed previousAdmin, address indexed newAdmin);
    event DecayRateUpdated(uint256 oldRate, uint256 newRate);
    event TokensDecayed(address indexed account, uint256 amount);

    // ============ STATE ============
    string private _name;
    string private _symbol;
    uint8 private constant _DECIMALS = 18;
    uint256 public totalSupply;

    mapping(address => uint256) private _balances;
    mapping(address => mapping(address => uint256)) private _allowances;
    mapping(address => uint256) private _lastUpdate; // timestamp of last decay application

    uint256 public decayRate; // per second, expressed with DECAY_PRECISION
    uint256 public constant DECAY_PRECISION = 1e18;
    uint256 public constant MAX_DECAY_RATE = DECAY_PRECISION / 10; // max 10% per second

    address public admin;
    address public minter;

    // ============ MODIFIERS ============
    modifier onlyAdmin() {
        if (msg.sender != admin) revert NeunodeToken__Unauthorized();
        _;
    }

    modifier onlyMinter() {
        if (msg.sender != minter) revert NeunodeToken__Unauthorized();
        _;
    }

    // ============ CONSTRUCTOR ============
    /**
     * @param name_ Token name
     * @param symbol_ Token symbol
     * @param decayRate_ Initial decay rate (per second, 1e18 = 100%/s)
     */
    constructor(
        string memory name_,
        string memory symbol_,
        uint256 decayRate_
    ) {
        if (decayRate_ > MAX_DECAY_RATE) revert NeunodeToken__InvalidDecayRate();

        _name = name_;
        _symbol = symbol_;
        admin = msg.sender;
        minter = msg.sender;
        decayRate = decayRate_;
    }

    // ============ VIEW FUNCTIONS ============
    function name() external view returns (string memory) { return _name; }
    function symbol() external view returns (string memory) { return _symbol; }
    function decimals() external pure returns (uint8) { return _DECIMALS; }

    /**
     * @notice Returns the current decay‑adjusted balance for an account.
     * @dev State is NOT modified. The returned value is the raw balance minus
     *      accrued decay up to this block.
     */
    function balanceOf(address account) external view returns (uint256) {
        if (_balances[account] == 0) return 0;
        uint256 rawBalance = _balances[account];
        uint256 delta = block.timestamp - _lastUpdate[account];
        if (delta == 0 || decayRate == 0) return rawBalance;

        uint256 decayAmount = (rawBalance * decayRate * delta) / DECAY_PRECISION;
        if (decayAmount > rawBalance) return 0;
        return rawBalance - decayAmount;
    }

    /**
     * @notice Returns the raw (undecayed) balance stored for an account.
     */
    function rawBalanceOf(address account) external view returns (uint256) {
        return _balances[account];
    }

    function allowance(address owner, address spender) external view returns (uint256) {
        return _allowances[owner][spender];
    }

    // ============ STATE FUNCTIONS ============

    /**
     * @notice Transfers tokens from caller to recipient.
     * @dev Applies decay on sender before transfer.
     */
    function transfer(address to, uint256 amount) external returns (bool) {
        if (to == address(0)) revert NeunodeToken__ZeroAddress();
        _applyDecay(msg.sender);
        if (_balances[msg.sender] < amount) revert NeunodeToken__InsufficientBalance();

        _balances[msg.sender] -= amount;
        _balances[to] += amount;
        _lastUpdate[to] = block.timestamp;
        _lastUpdate[msg.sender] = block.timestamp;
        emit Transfer(msg.sender, to, amount);
        return true;
    }

    /**
     * @notice Transfers tokens on behalf of `from` to `to`.
     * @dev Requires allowance. Applies decay on `from` before transfer.
     */
    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        if (to == address(0)) revert NeunodeToken__ZeroAddress();
        if (from == address(0)) revert NeunodeToken__ZeroAddress();

        uint256 currentAllowance = _allowances[from][msg.sender];
        if (currentAllowance != type(uint256).max) {
            if (currentAllowance < amount) revert NeunodeToken__InsufficientAllowance();
            unchecked { _allowances[from][msg.sender] = currentAllowance - amount; }
        }

        _applyDecay(from);
        if (_balances[from] < amount) revert NeunodeToken__InsufficientBalance();

        _balances[from] -= amount;
        _balances[to] += amount;
        _lastUpdate[to] = block.timestamp;
        _lastUpdate[from] = block.timestamp;
        emit Transfer(from, to, amount);
        return true;
    }

    /**
     * @notice Approves spender to spend `amount` tokens on caller's behalf.
     */
    function approve(address spender, uint256 amount) external returns (bool) {
        if (spender == address(0)) revert NeunodeToken__ZeroAddress();
        _allowances[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    // ============ MINTER / ADMIN FUNCTIONS ============

    /**
     * @notice Mints new tokens to `to` (resource‑backed).
     * @dev Only callable by the minter role. Sets lastUpdate timestamp for recipient.
     */
    function mint(address to, uint256 amount) external onlyMinter {
        if (to == address(0)) revert NeunodeToken__ZeroAddress();
        if (amount == 0) return;

        _balances[to] += amount;
        totalSupply += amount;
        _lastUpdate[to] = block.timestamp;
        emit Transfer(address(0), to, amount);
    }

    /**
     * @notice Burns `amount` tokens from caller's balance.
     * @dev Applies decay first, then burns remaining tokens.
     */
    function burn(uint256 amount) external {
        _applyDecay(msg.sender);
        if (_balances[msg.sender] < amount) revert NeunodeToken__InsufficientBalance();

        _balances[msg.sender] -= amount;
        totalSupply -= amount;
        _lastUpdate[msg.sender] = block.timestamp;
        emit Transfer(msg.sender, address(0), amount);
    }

    /**
     * @notice Burns `amount` tokens from `account`, using allowance.
     * @dev Applies decay on `account` first. Requires caller allowance.
     */
    function burnFrom(address account, uint256 amount) external {
        if (account == address(0)) revert NeunodeToken__ZeroAddress();

        uint256 currentAllowance = _allowances[account][msg.sender];
        if (currentAllowance != type(uint256).max) {
            if (currentAllowance < amount) revert NeunodeToken__InsufficientAllowance();
            unchecked { _allowances[account][msg.sender] = currentAllowance - amount; }
        }

        _applyDecay(account);
        if (_balances[account] < amount) revert NeunodeToken__InsufficientBalance();

        _balances[account] -= amount;
        totalSupply -= amount;
        _lastUpdate[account] = block.timestamp;
        emit Transfer(account, address(0), amount);
    }

    /**
     * @notice Updates the decay rate (per second, 1e18 = 100%/s).
     * @dev Max allowed is 10%/s.
     */
    function setDecayRate(uint256 newRate) external onlyAdmin {
        if (newRate > MAX_DECAY_RATE) revert NeunodeToken__InvalidDecayRate();
        uint256 oldRate = decayRate;
        decayRate = newRate;
        emit DecayRateUpdated(oldRate, newRate);
    }

    /**
     * @notice Transfers admin role.
     */
    function setAdmin(address newAdmin) external onlyAdmin {
        if (newAdmin == address(0)) revert NeunodeToken__ZeroAddress();
        address oldAdmin = admin;
        admin = newAdmin;
        emit AdminChanged(oldAdmin, newAdmin);
    }

    /**
     * @notice Transfers minter role.
     */
    function setMinter(address newMinter) external onlyAdmin {
        if (newMinter == address(0)) revert NeunodeToken__ZeroAddress();
        address oldMinter = minter;
        minter = newMinter;
        emit MinterChanged(oldMinter, newMinter);
    }

    // ============ INTERNAL ============

    /**
     * @dev Applies decay to `account` balance and updates state.
     *      Called before any operation that reduces the sender's balance.
     */
    function _applyDecay(address account) internal {
        uint256 rawBalance = _balances[account];
        if (rawBalance == 0) {
            _lastUpdate[account] = block.timestamp;
            return;
        }

        uint256 delta = block.timestamp - _lastUpdate[account];
        if (delta == 0 || decayRate == 0) return;

        uint256 decayAmount = (rawBalance * decayRate * delta) / DECAY_PRECISION;
        if (decayAmount > rawBalance) decayAmount = rawBalance;

        if (decayAmount > 0) {
            _balances[account] = rawBalance - decayAmount;
            totalSupply -= decayAmount;
            emit TokensDecayed(account, decayAmount);
            emit Transfer(account, address(0), decayAmount);
        }

        _lastUpdate[account] = block.timestamp;
    }
}