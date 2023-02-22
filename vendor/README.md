# Third party packages

This directory contatins packages from third-party vendors reused in Astar Network.

### Why not fork?

As a way to protect code from **unexpected changes** and **release from third-party project
internal dependencies** a dedicated folder used instead of repository fork.

In case of internal packages the porter persion will have no intention to just scroll down
repository commit history and adjust final result to the repo. Copy/paste the code risk is
still exist, but in this case all changes will be clearly viewed in PR created before merge.

## Package list

| Directory                          | Package name                   | Origin                                        |
|------------------------------------|--------------------------------|-----------------------------------------------|
| evm-tracing                        | moonbeam-client-evm-tracing    | ${moonbeam}/client/evm-tracing                | 
| rpc/debug                          | moonbeam-rpc-debug             | ${moonbeam}/client/rpc/debug                  |
| rpc/trace                          | moonbeam-rpc-trace             | ${moonbeam}/client/rpc/trace                  |
| rpc/txpool                         | moonbeam-rpc-txpool            | ${moonbeam}/client/rpc/txpool                 |
| rpc-core/types                     | moonbeam-rpc-core-types        | ${moonbeam}/client/rpc-core/types             |
| rpc-core/debug                     | moonbeam-rpc-core-debug        | ${moonbeam}/client/rpc-core/debug             |
| rpc-core/trace                     | moonbeam-rpc-core-trace        | ${moonbeam}/client/rpc-core/trace             |
| rpc-core/txpool                    | moonbeam-rpc-core-txpool       | ${moonbeam}/client/rpc-core/txpool            |
| runtime/evm_tracer                 | moonbeam-evm-tracer            | ${moonbeam}/runtime/evm_tracer                |
| runtime/ext                        | moonbeam-primitives-ext        | ${moonbeam}/primitives/ext                    |
| primitives/evm-tracing-events      | evm-tracing-events             | ${moonbeam}/primitives/rpc/evm-tracing-events |
| primitives/debug                   | moonbeam-rpc-primitives-debug  | ${moonbeam}/primitives/rpc/debug              |
| primitives/txpool                  | moonbeam-rpc-primitives-txpool | ${moonbeam}/primitives/rpc/txpool             |

