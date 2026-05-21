#!/usr/bin/env bash
# Start solana-test-validator with all 5 Janus programs preloaded,
# wait for it to come up, run the Node E2E test, then tear down.
#
# Prereqs:
#   - `cargo build-sbf` has been run (target/deploy/janus_*.so exist)
#   - `cd sdk && pnpm install && pnpm build`
#   - `cd scripts/e2e-localnet && pnpm install`

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

VALIDATOR_LOG=$(mktemp /tmp/janus-validator.XXXXXX.log)
LEDGER_DIR=$(mktemp -d /tmp/janus-ledger.XXXXXX)
echo "validator log: $VALIDATOR_LOG"
echo "ledger dir:    $LEDGER_DIR"

cleanup() {
  echo
  echo "--- shutting down validator ---"
  if [[ -n "${VALIDATOR_PID:-}" ]]; then
    kill "$VALIDATOR_PID" 2>/dev/null || true
    wait "$VALIDATOR_PID" 2>/dev/null || true
  fi
  rm -rf "$LEDGER_DIR"
}
trap cleanup EXIT

CT=$(solana-keygen pubkey target/deploy/janus_conditional_tokens-keypair.json)
LM=$(solana-keygen pubkey target/deploy/janus_lmsr_market-keypair.json)
SHR=$(solana-keygen pubkey target/deploy/janus_slot_height_resolver-keypair.json)
PPR=$(solana-keygen pubkey target/deploy/janus_pyth_price_resolver-keypair.json)
MF=$(solana-keygen pubkey target/deploy/janus_market_factory-keypair.json)

echo "--- starting solana-test-validator (programs deployed after boot) ---"
solana-test-validator --reset --quiet \
  --ledger "$LEDGER_DIR" \
  > "$VALIDATOR_LOG" 2>&1 &
VALIDATOR_PID=$!
echo "validator pid: $VALIDATOR_PID"

# Wait for RPC to come up.
echo -n "waiting for validator RPC"
for i in $(seq 1 60); do
  if curl -sS http://127.0.0.1:8899 -X POST -H "Content-Type: application/json" \
      -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
      | grep -q '"result":"ok"'; then
    echo " ready"
    break
  fi
  echo -n "."
  sleep 0.5
done

# Make sure the deployer wallet has SOL.
solana --url http://127.0.0.1:8899 airdrop 100 > /dev/null 2>&1 || true

echo "--- deploying programs via `solana program deploy` ---"
deploy_program() {
  local name=$1
  local keypair=$2
  echo "  deploying $name from target/deploy/${name}.so with keypair $keypair"
  solana --url http://127.0.0.1:8899 program deploy \
    --program-id "$keypair" \
    "target/deploy/${name}.so"
}
deploy_program janus_conditional_tokens   target/deploy/janus_conditional_tokens-keypair.json
deploy_program janus_lmsr_market           target/deploy/janus_lmsr_market-keypair.json
deploy_program janus_slot_height_resolver  target/deploy/janus_slot_height_resolver-keypair.json
deploy_program janus_pyth_price_resolver   target/deploy/janus_pyth_price_resolver-keypair.json
deploy_program janus_market_factory        target/deploy/janus_market_factory-keypair.json

# Confirm all 5 programs are loaded.
for id in "$CT" "$LM" "$SHR" "$PPR" "$MF"; do
  if ! solana --url http://127.0.0.1:8899 account "$id" > /dev/null 2>&1; then
    echo "ERROR: program $id not loaded after deploy"
    exit 1
  fi
done
echo "all 5 programs deployed ✓"

echo
echo "--- running e2e test ---"
cd scripts/e2e-localnet
node test.mjs
