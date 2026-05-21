#!/usr/bin/env bash
# Deploy all 5 Janus programs to devnet (or any RPC the caller's
# `solana` CLI is pointed at). Idempotent: if a program is already
# deployed at its keypair-derived address, the deploy is a no-op.
#
# Usage:
#   scripts/devnet/deploy.sh                          # uses solana CLI's configured RPC
#   solana config set --url devnet && bash scripts/devnet/deploy.sh
#
# Prereqs:
#   - solana CLI configured with a funded wallet
#   - cargo build-sbf has been run (target/deploy/janus_*.so exist)
#   - target/deploy/janus_*-keypair.json exists (run sync-program-ids.sh)

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

RPC=${JANUS_RPC:-$(solana config get json_rpc_url | awk -F': ' '{print $2}')}
WALLET=$(solana address)
echo "RPC:    $RPC"
echo "wallet: $WALLET"
echo "balance: $(solana --url "$RPC" balance "$WALLET")"
echo

# Each program: deploy if not already on-chain, otherwise skip.
deploy_one() {
  local name=$1
  local kp="target/deploy/${name}-keypair.json"
  local so="target/deploy/${name}.so"
  local id
  id=$(solana-keygen pubkey "$kp")

  if solana --url "$RPC" program show "$id" > /dev/null 2>&1; then
    echo "✓ $name already deployed at $id — skipping"
  else
    echo "→ deploying $name to $id"
    solana --url "$RPC" program deploy --program-id "$kp" "$so"
  fi
}

deploy_one janus_conditional_tokens
deploy_one janus_lmsr_market
deploy_one janus_lmsr_true_market
deploy_one janus_slot_height_resolver
deploy_one janus_pyth_price_resolver
deploy_one janus_market_factory

echo
echo "All programs deployed. Next:"
echo "  JANUS_RPC=$RPC node scripts/e2e-localnet/test.mjs"
echo "  (the e2e test airdrops SOL — on devnet it'll use whatever's in your wallet instead)"
