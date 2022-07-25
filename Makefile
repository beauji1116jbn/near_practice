.PHONY: lint build test

lint:
	cd ft && make lint
	cd amm_wallet && make lint
	cd amm && make lint
	cd integration-tests && make lint

build: lint
	cd ft && make build
	cd amm_wallet && make build
	cd amm && make build

test:
	cd integration-tests && make test