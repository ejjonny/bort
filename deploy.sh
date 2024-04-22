export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc
cargo build --bin main --release --target=x86_64-unknown-linux-gnu
ssh -i /Users/ejohn/.ssh/id_rsa root@137.184.37.41 'tmux send-keys C-c'
scp target/x86_64-unknown-linux-gnu/release/main root@137.184.37.41:/root/brt/
ssh -i /Users/ejohn/.ssh/id_rsa root@137.184.37.41 'tmux send-keys "./main" Enter'