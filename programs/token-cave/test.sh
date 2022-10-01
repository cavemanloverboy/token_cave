solana-test-validator -r --bpf-program "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS" ../../target/deploy/token_cave.so > test-validator.log &
sleep 5
cargo test