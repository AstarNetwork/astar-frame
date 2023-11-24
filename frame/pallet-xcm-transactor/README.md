# XCM CE + Companion Pallet
_XCM Chain extension with callback functionality_

> This is WIP draft implementation with a lot of pending TODOs and security considerations still to be addressed.

## Chain Extension

The CE has following commands and [SDK for ink! contracts](frame/pallet-xcm-transactor/ink-sdk).
```rust
pub enum Command {
    /// Returns the weight for given XCM and saves it (in CE, per-call scratch buffer) for
    /// execution
    PrepareExecute = 0,
    /// Execute the XCM that was prepared earlier
    Execute = 1,
    /// Returns the fee required to send XCM and saves it for sending
    ValidateSend = 2,
    /// Send the validated XCM
    Send = 3,
    /// Register the new query
    NewQuery = 4,
    /// Take the response for query if available, in case of no callback query
    TakeResponse = 5,
    /// Get the pallet account id which will be the caller of contract callback
    PalletAccountId = 6,
}

```

## Callback Design
The callback design make use of `pallet_xcm`'s `OnResponse` handler which has capability to notify a dispatch on a XCM response (if notify query is registered).

For us that dispatch is companion pallet's `on_callback_received` which will route the xcm response (`Response` enum) back to contract via a `bare_call` (if wasm contract)

![image](https://user-images.githubusercontent.com/17181457/236989729-acf5ac13-4abe-4340-bcdc-6ca22fb5d411.png)


## Structure
```
├── pallet-xcm-transactor
│   ├── contract-examples       # contract examples using XCM CE, some of which are used as fixtures in tests
│   ├── ink-sdk                 # ink helper methods to build CE calls and export types
│   ├── primitives              # common types to share with pallet and CE
│   ├── src                     # companion pallet, for callback support
│   └── xcm-simulator           # xcm simulator, for testing XCM CE
```


## Local testing
All the test scenarios are done inside XCM Simulator - [here](frame/pallet-xcm-transactor/xcm-simulator/src/lib.rs)

### Run tests
- cd into xcm simulator directory
  ```
  cd frame/pallet-xcm-transactor/xcm-simulator
  ```
- `cargo test` - it will take a while for first time since it needs to build the contracts too

To print the XCM logs, use the below command
```
RUST_LOG="xcm=trace" cargo test  --  --nocapture --test-threads=1
```

### To add new contract to fixtures
1. Create the contract inside [`contract-examples`](frame/pallet-xcm-transactor/contract-examples) directory
2. Add the contract in [`build.rs`](frame/pallet-xcm-transactor/xcm-simulator/build.rs) of simulator tests so that contract will be compiled and copied to fixtures dir before test runs.
   ```
    build_contract(
        &fixtures_dir,
        &contracts_dir.join("YOUR_CONTRACT"),
        "YOUR_CONTRACT",
    );
   ```

See the existing tests to know how fixtures are used.
