.PHONY: lint build

lint:
	cd ft && make lint
	cd amm_wallet && make lint
	cd amm && make lint

build: lint
	cd ft && make build
	cd amm_wallet && make build
	cd amm && make build