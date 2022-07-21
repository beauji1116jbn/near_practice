All: deploy

.PHONY: deploy build lint

build:
	./scripts/build.sh

deploy: build
	./scripts/deploy.sh

lint:
	cargo clippy --fix --allow-dirty --allow-staged