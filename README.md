# BeamMP Rust Client

A client to aid development of any BeamMP Server.

Examples of how to run it:

```shell
# connect to localhost:30814
BEAMMP_USER="myepicusername123" BEAMMP_PASS="secret password 23$" ./beammp_rust_client localhost 30814

# connect to 1.1.1.1:30814
BEAMMP_USER="myepicusername123" BEAMMP_PASS="secret password 23$" ./beammp_rust_client 1.1.1.1 30814

# build & connect to localhost:30814
BEAMMP_USER="myepicusername123" BEAMMP_PASS="secret password 23$" cargo run -- localhost 30814
```