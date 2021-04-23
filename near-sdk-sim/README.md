# NEAR Simulator & cross-contract testing library

When writing NEAR contracts, with Rust or other Wasm-compiled languages like [AssemblyScript](https://github.com/near/near-sdk-as), the default testing approach for your language of choice (such as `mod test` in your Rust project's `src/lib.rs` file) is great for testing the behavior of a single contract in isolation.

But the true power of blockchains & smart contracts comes from cross-contract calls. How do you make sure your cross-contract code works as you expect?

As a first step, you can use this library! With it, you can:

- Test cross-contract calls
- Profile [gas](https://docs.near.org/docs/concepts/gas) & [storage](https://docs.near.org/docs/concepts/storage-staking) usage for your contract, establishing lower bounds for costs of deployed contracts and rapidly identifying problematic areas prior to deploying.
- Inspect intermediate state of all calls in a complicated chain of transactions

To view this documentation locally, clone this repo and from this folder run `cargo doc --open`.

# Getting started

This section will guide you through our suggested approach to adding simulation tests to your project. Want an example? Check out the [Fungible Token Example](https://github.com/near/near-sdk-rs/tree/master/examples/fungible-token).

## Dependency versions

Currently this crate depends on a the GitHub repo of [nearcore](https://github.com/near/nearcore), so this crate must be a git dependency too. Furthermore, this crate's dependencies conflict with building the Wasm smart contract, so you must add it under the following:

```toml
[dev-dependencies]
near-sdk-sim = "=3.1.0"

```

And update `near-sdk` too:

```toml
[dependencies]
near-sdk = "=3.1.0"

```

Note that you need to add the `tag` (or `commit`) of the version.

## Workspace setup

If you want to check gas & storage usage of one Rust contract, you can add the above dependencies to `Cargo.toml` in the root of your project. If you want to test cross-contract calls, we recommend setting up a cargo [workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html). Here's how it works:

Let's say you have an existing contract project in a folder called `contract`.

Go ahead and make a subfolder within it called `contract`, and move the original contents of `contract` into this subfolder. Now you'll have `contract/contract`. You can rename the root folder to something like `contracts` or `contract-wrap`, if you want. Some bash commands to do this:

```bash
mkdir contract-wrap
mv contract contract-wrap
```

Now in the root of the project (`contract-wrap`), create a new `Cargo.toml`. You'll have to add the normal `[package]` section, but unlike most projects you won't have any `dependencies`, only `dev-dependencies` and a `workspace`:

```toml
[dev-dependencies]
near-sdk = "=3.1.0"
near-sdk-sim = "=3.1.0"
contract = { path = "./contract" }

[workspace]
members = [
  "contract"
]
```

Now when you want to create test contracts, you can add a new subfolder to your project and add a line for it to both `[dev-dependencies]` and `[workspace]` in this root `Cargo.toml`.

Other cleanup:

- You can move any `[profile.release]` settings from your nested project up to the root of the workspace, since workspace members inherit these settings from the workspace root.
- You can remove the nested project's `target`, since all workspace members will be built to the root project's `target` directory
- If you were building with `cargo build`, you can now build all workspace members at once with `cargo build --workspace`

## Test files

In the root of your project (`contract-wrap` in the example above), create a `tests` directory with a Rust file inside. Anything in here will automatically be run by `cargo test`.

Inside this folder, set up a new test crate for yourself by creating a `tests/sim` directory with a `tests/sim/main.rs` file. This file will glue together the other files (aka modules) in this folder. We'll add things to it soon. For now you can leave it empty.

Now create a `tests/sim/utils.rs` file. This file will export common functions for all your tests. In it you need to include the bytes of the contract(s) you want to test:

```rust
near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    // update `contract.wasm` for your contract's name
    CONTRACT_WASM_BYTES => "target/wasm32-unknown-unknown/release/contract.wasm",

    // if you run `cargo build` without `--release` flag:
    CONTRACT_WASM_BYTES => "target/wasm32-unknown-unknown/debug/contract.wasm",
}
```

Note that this means **you must `build` before you `test`!** Since `cargo test` does not re-generate the `wasm` files that your simulation tests rely on, you will need to `cargo build --all --target wasm32-unknown-unknown` before running `cargo test`. If you made contract changes and _you swear it should pass now_, try rebuilding!

Now you can make a function to initialize your simulator:

```rust
use near_sdk_sim::{init_simulator, to_yocto, STORAGE_AMOUNT};

const CONTRACT_ID: &str = "contract";

pub fn init() -> (UserAccount, UserAccount, UserAccount) {
    // Use `None` for default genesis configuration; more info below
    let root = init_simulator(None);

    let contract = root.deploy(
        &CONTRACT_WASM_BYTES,
        CONTRACT_ID.to_string(),
        STORAGE_AMOUNT // attached deposit
    );

    let alice = root.create_user(
        "alice".to_string(),
        to_yocto("100") // initial balance
    );

    (root, contract, alice)
}
```

Now you can add a test file that uses this `init` function in `tests/sim/first_tests.rs`. For every file you add to this directory, you'll need to add a line to `tests/sim/main.rs`. Let's add one for both files so far:

```rust
// in tests/sim/main.rs
mod utils;
mod first_tests;
```

Now add some tests to `first_tests.rs`:

```rust
use near_sdk::serde_json::json;
use near_sdk_sim::DEFAULT_GAS;

use crate::utils::init;

#[test]
fn simulate_some_view_function() {
    let (root, contract, _alice) = init();

    let actual: String = root.view(
        contract.account_id(),
        "view_something",
        &json!({
            "some_param": "some_value".to_string(),
        }).to_string().into_bytes(),
    ).unwrap_json();

    assert_eq!("expected".to_string(), actual);
}

#[test]
fn simulate_some_change_method() {
    let (root, contract, _alice) = init();

    let result = root.call(
        contract.account_id(),
        "change_something",
        json!({
            "some_param": "some_value".to_string(),
        }).to_string().into_bytes(),
        DEFAULT_GAS,
        1, // deposit
    );

    assert!(result.is_ok());
}
```

## Optional macros

The above approach is a good start, and will work even if your Wasm files are compiled from a language other than Rust.

But if your original files are Rust and you want better ergonomics while testing, `near-sdk-sim` provides a nice bonus feature.

`near-sdk-sim` modifies the `near_bindgen` macro from `near-sdk` to create an additional struct+implementation from your contract, with `Contract` added to the end of the name, like `xxxxxContract`. So if you have a contract with `[package].name` set to `token` with this in its `src/lib.rs`:

```rust
#[near_bindgen]
struct Token {
    ...
}

#[near_bindgen]
impl Token {
    ...
}
```

Then in your simulation test you can import `TokenContract`:

```rust
use token::TokenContract;

// or rename it maybe
use token::TokenContract as OtherNamedContract;
```

Now you can simplify the `init` & test code from the previous section:

```rust
// in utils.rs
use near_sdk_sim::{deploy, init_simulator, to_yocto, STORAGE_AMOUNT};
use token::TokenContract;

const CONTRACT_ID: &str = "contract";

pub fn init() -> (UserAccount, ContractAccount<TokenContract>, UserAccount) {
    let root = init_simulator(None);

    let contract = deploy!(
        contract: TokenContract,
        contract_id: CONTRACT_ID,
        bytes: &CONTRACT_WASM_BYTES,
        signer_account: root,
    );

    let alice = root.create_user(
        "alice".to_string(),
        to_yocto("100") // initial balance
    );

    (root, contract, alice)
}

// in first_tests.rs
use near_sdk_sim::{call, view};
use crate::utils::init;

#[test]
fn simulate_some_view_function() {
    let (root, contract, _alice) = init();

    let actual: String = view!(
        contract.view_something("some_value".to_string()),
    ).unwrap_json();

    assert_eq!("expected", actual);
}

#[test]
fn simulate_some_change_method() {
    let (root, contract, _alice) = init();

    // uses default gas amount
    let result = call!(
        root,
        contract.change_something("some_value".to_string()),
        deposit = 1,
    );

    assert!(result.is_ok());
}
```

# Common patterns

## Profile gas costs

For a chain of transactions kicked off by `call` or `call!`, you can check the `gas_burnt` and `tokens_burnt`, where `tokens_burnt` will equal `gas_burnt` multiplied by the `gas_price` set in the genesis config. You can also print out `profile_data` to see an in-depth gas-use breakdown.

```rust
let outcome = some_account.call(
    "some_contract",
    "method",
    &json({
        "some_param": "some_value",
    }).to_string().into_bytes(),
    DEFAULT_GAS,
    0,
);

println!(
    "profile_data: {:#?} \n\ntokens_burnt: {}Ⓝ",
    outcome.profile_data(),
    (outcome.tokens_burnt()) as f64 / 1e24
);

let expected_gas_ceiling = 5 * u64::pow(10, 12); // 5 TeraGas
assert!(outcome.gas_burnt() < expected_gas_ceiling);
```

TeraGas units are [explained here](https://docs.near.org/docs/concepts/gas#thinking-in-gas).

Remember to run tests with `--nocapture` to see output from `println!`:

    cargo test -- --nocapture

The output from this `println!` might look something like this:

    profile_data: ------------------------------
    Total gas: 1891395594588
    Host gas: 1595600369775 [84% total]
    Action gas: 0 [0% total]
    Wasm execution: 295795224813 [15% total]
    ------ Host functions --------
    base -> 7678275219 [0% total, 0% host]
    contract_compile_base -> 35445963 [0% total, 0% host]
    contract_compile_bytes -> 48341969250 [2% total, 3% host]
    read_memory_base -> 28708495200 [1% total, 1% host]
    read_memory_byte -> 634822611 [0% total, 0% host]
    write_memory_base -> 25234153749 [1% total, 1% host]
    write_memory_byte -> 539306856 [0% total, 0% host]
    read_register_base -> 20137321488 [1% total, 1% host]
    read_register_byte -> 17938284 [0% total, 0% host]
    write_register_base -> 25789702374 [1% total, 1% host]
    write_register_byte -> 821137824 [0% total, 0% host]
    utf8_decoding_base -> 3111779061 [0% total, 0% host]
    utf8_decoding_byte -> 15162184908 [0% total, 0% host]
    log_base -> 3543313050 [0% total, 0% host]
    log_byte -> 686337132 [0% total, 0% host]
    storage_write_base -> 192590208000 [10% total, 12% host]
    storage_write_key_byte -> 1621105941 [0% total, 0% host]
    storage_write_value_byte -> 2047223574 [0% total, 0% host]
    storage_write_evicted_byte -> 2119742262 [0% total, 0% host]
    storage_read_base -> 169070537250 [8% total, 10% host]
    storage_read_key_byte -> 711908259 [0% total, 0% host]
    storage_read_value_byte -> 370326330 [0% total, 0% host]
    touching_trie_node -> 1046627135190 [55% total, 65% host]
    ------ Actions --------
    ------------------------------


    tokens_burnt: 0.00043195379520539996Ⓝ

## Profile [storage](https://docs.near.org/docs/concepts/storage-staking) costs

For a `ContractAccount` created with `deploy!` or a `UserAccount` created with `root.create_user`, you can call `account()` to get the [Account](https://github.com/near/nearcore/blob/df2d8bac977461c3abded5ef52ac3000f53e9097/core/primitives-core/src/account.rs#L8-L21) information stored in the simulated blockchain.

```rs
let account = root.account().unwrap();
let balance = account.amount;
let locked_in_stake = account.locked;
let storage_usage = account.storage_usage;
```

You can use this info to do detailed profiling of how contract calls alter the storage usage of accounts.

## Inspect intermediate state of all calls in a complicated chain of transactions

Say you have a `call` or `call!`:

```rust
let outcome = some_account.call(
    "some_contract",
    "method",
    &json({
        "some_param": "some_value",
    }).to_string().into_bytes(),
    DEFAULT_GAS,
    0,
);
```

If `some_contract.method` here makes cross-contract calls, `near-sdk-sim` will allow all of these calls to complete. You can then inspect the entire chain of calls via the `outcome` struct. Some useful methods:

- [`outcome.promise_results()`](https://github.com/near/near-sdk-rs/blob/9cf75cf4a537a6f9906d82cfcadd97ae4a3443b6/near-sdk-sim/src/outcome.rs#L123-L126)
- [`outcome.get_receipt_results()`](https://github.com/near/near-sdk-rs/blob/9cf75cf4a537a6f9906d82cfcadd97ae4a3443b6/near-sdk-sim/src/outcome.rs#L114-L117)
- [`outcome.logs()`](https://github.com/near/near-sdk-rs/blob/9cf75cf4a537a6f9906d82cfcadd97ae4a3443b6/near-sdk-sim/src/outcome.rs#L156-L159)

You can use these with `println!` and [pretty print interpolation](https://riptutorial.com/rust/example/1248/advanced-usage-of-println-):

```rust
println!("{:#?}", outcome.promise_results);
```

Remember to run your tests with `--nocapture` to see the `println!` output:

    cargo test -- --nocapture

You might see something like this:

    [
        Some(
            ExecutionResult {
                outcome: ExecutionOutcome {
                    logs: [],
                    receipt_ids: [
                        `2bCDBfWgRkzGggXLuiXqhnVGbxwRz7RP3qa8WS5nNw8t`,
                    ],
                    burnt_gas: 2428220615156,
                    tokens_burnt: 0,
                    status: SuccessReceiptId(2bCDBfWgRkzGggXLuiXqhnVGbxwRz7RP3qa8WS5nNw8t),
                },
            },
        ),
        Some(
            ExecutionResult {
                outcome: ExecutionOutcome {
                    logs: [],
                    receipt_ids: [],
                    burnt_gas: 18841799405111,
                    tokens_burnt: 0,
                    status: Failure(Action #0: Smart contract panicked: panicked at 'Not an integer: ParseIntError { kind: InvalidDigit }', test-contract/src/lib.rs:85:56),
                },
            },
        )
    ]

You can see it's a little hard to tell which call is which, since the [ExecutionResult](https://github.com/near/near-sdk-rs/blob/9cf75cf4a537a6f9906d82cfcadd97ae4a3443b6/near-sdk-sim/src/outcome.rs#L20-L27) does not yet include the name of the contract or method. To help debug, you can use `log!` in your contract methods. All `log!` output will show up in the `logs` arrays in the ExecutionOutcomes shown above.

## Check expected transaction failures

If you want to check something in the `logs` or `status` of one of the transactions in one of these call chains mentioned above, you can use string matching. To check that the Failure above matches your expectations, you could:

```rust
use near_sdk_sim::transaction::ExecutionStatus;

#[test]
fn simulate_some_failure() {
    let outcome = some_account.call(...);

    assert_eq!(res.promise_errors().len(), 1);

    if let ExecutionStatus::Failure(execution_error) =
        &outcome.promise_errors().remove(0).unwrap().outcome().status
    {
        assert!(execution_error.to_string().contains("ParseIntError"));
    } else {
        unreachable!();
    }
}
```

This [`promise_errors`](https://github.com/near/near-sdk-rs/blob/9cf75cf4a537a6f9906d82cfcadd97ae4a3443b6/near-sdk-sim/src/outcome.rs#L128-L135) returns a filtered version of the `promise_results` method mentioned above.

Parsing `logs` is much simpler, whether [from `get_receipt_results`](https://github.com/near/near-sdk-rs/blob/9cf75cf4a537a6f9906d82cfcadd97ae4a3443b6/examples/fungible-token/tests/sim/with_macros.rs#L128-L134) or [from `logs` directly](https://github.com/near/near-sdk-rs/blob/9cf75cf4a537a6f9906d82cfcadd97ae4a3443b6/examples/fungible-token/tests/sim/with_macros.rs#L70-L74).

# Tweaking the genesis config

For many simulation tests, using `init_simulator(None)` is good enough. This uses the [default genesis configuration settings](https://github.com/near/near-sdk-rs/blob/0a9a56f1590e1f19efc974160c88f32efcb91ef4/near-sdk-sim/src/runtime.rs#L59-L72):

```rust
GenesisConfig {
    genesis_time: 0,
    gas_price: 100_000_000,
    gas_limit: std::u64::MAX,
    genesis_height: 0,
    epoch_length: 3,
    runtime_config: RuntimeConfig::default(),
    state_records: vec![],
    validators: vec![],
}
```

If you want to override some of these values, for example to simulate how your contract will behave if [gas price](https://docs.near.org/docs/concepts/gas) goes up 10x:

```rs
use near_sdk_sim::runtime::GenesisConfig;

pub fn init () {
    let mut genesis = GenesisConfig::default();
    genesis.gas_price = genesis.gas_price * 10;
    let root = init_simulator(Some(genesis));
}
```

## Loading dumped state from testnet/mainnet contract

This is useful when your test depends on real contract state that exists on testnet and mainnet. For example, storage cost and some gas cost on testnet is depends on the real size of the state that your contract used on testnet and mainnet. So download and start simulator with them gives you a more accurate understanding of gas/storage cost. Another use case is you'd like to migrate your contract state format from old struct defintion to new, in this case fetch testnet states and run integration tests locally will give you confident that your migration works without breaking contract methods.

First you'll need a tool to download state of given contract. Note, the `view_state` rpc doesn't give you the same format that you can passed to simulator as initial state. The major difference is intitial state need every state record typed and has its account id. You can use this [script](https://github.com/near/repro-near-funcall/blob/master/collect-state-records.js) to download multiple contract account state, at given height, in a format that can be used as initial state of near-sdk-sim:

```
# latest block height:
node collect-state-records.js -a contract1.testnet contract2.testnet > state_records.json
# given block height 111111:
node collect-state-records.js -a contract1.testnet contract2.testnet -b 111111 > state_records.json
# mainnet:
node collect-state-records.js -a contract1.near contract2.near -u https://rpc.near.org > state_records.json
```

Now you can load `state_records.json`, which contains all state of your specified contracts in near-sdk-sim, by set it in `GenesisConfig`:

```rust
pub fn init () {
    let genesis = GenesisConfig::default()
        // must have contract accounts exist, otherwise state_records are invalid and init runtime will fail
        .add_account("contract1.testnet", 10000000000000000000)
        .add_account("contract2.testnet", 10000000000000000000)
        .load_state_records("./state_records.json");
    let root = init_simulator(Some(genesis));
}
```
