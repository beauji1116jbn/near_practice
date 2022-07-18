#!/usr/bin/env sh

TEST_ACCOUNT=
near deploy --wasmFile res/fungible_token.wasm --accountId $TEST_ACCOUNT<<-EOF
y
EOF

near deploy --wasmFile res/fungible_token.wasm --accountId $TEST_ACCOUNT<<-EOF
y
EOF