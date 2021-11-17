test:
    cargo test
fmt:
    cargo fmt
lint: fmt
    cargo clippy
build:
    cargo build
run-ci: lint build test

has-dir:
	mkdir -p public
build-prod: has-dir
	mkdir -p public/js
	cd frontend && clojure -M:prod
copy-assets: has-dir
	cp frontend/resources/public/*css public
build-docker:
	docker build -t ralexstokes/ethereum_consensus_monitor .
push-docker:
	docker push ralexstokes/ethereum_consensus_monitor
deploy-docker: build-docker push-docker

run CONFIG:
    cargo run -- --config-path {{CONFIG}}
