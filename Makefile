all: release

dev: cargo build

dev_server:
	cargo run server ./seed &
	bash -c "trap 'pkill cook' EXIT; cd ./ui && npm run dev"

test: cargo test
