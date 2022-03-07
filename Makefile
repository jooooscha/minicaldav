test:
	cargo test --lib
it:
	cargo test --package minicaldav --test integration_test --  --nocapture --test-threads 1