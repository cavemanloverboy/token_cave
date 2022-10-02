# Token Cave: A Time-Locked SPL-Token Vault
A token cave allows users to deposit funds into a program-owned token pda. If a user wishes to withdraw, they must submit an unlock tx and wait the time specified at deposit (up to a week presently).

At deposit time, a user can can supply an `Option<Pubkey>`. If it is `None`, then the cave is in anti-wrench attack mode -- nobody can access funds during the time-lock. If it is `Some(key)`, then the cave is in hot wallet protection mode -- a user can supply an abort ix which sends the funds to the backup key's associated token account. This gives a user a safe savings account that gives them time to react and migrate funds when their key has been compromised.

To run tests, spin up a test validator via
```
solana-test-validator -r --bpf-program "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS" ./target/deploy/token_cave.so
```
and run
```
cargo test
```