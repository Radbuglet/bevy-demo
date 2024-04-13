run:
	cargo +nightly-2024-03-10 autoken check --old-artifacts delete
	cargo run --release
