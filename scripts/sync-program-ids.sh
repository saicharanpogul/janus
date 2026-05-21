#!/usr/bin/env bash
# Read the freshly-generated program keypairs in target/deploy and
# propagate the resulting pubkeys to every place that hardcodes a
# program ID:
#   - each program's `declare_id!` macro
#   - market-factory's cross-program CONDITIONAL_TOKENS_ID / LMSR_MARKET_ID
#   - sdk/src/constants.ts
#   - tests/src/lib.rs ids module
#
# Run after `solana-keygen new -o target/deploy/<program>-keypair.json`.
# Idempotent: re-running with the same keypairs is a no-op.

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

CT=$(solana-keygen pubkey target/deploy/janus_conditional_tokens-keypair.json)
LM=$(solana-keygen pubkey target/deploy/janus_lmsr_market-keypair.json)
LMT=$(solana-keygen pubkey target/deploy/janus_lmsr_true_market-keypair.json 2>/dev/null || echo "")
SHR=$(solana-keygen pubkey target/deploy/janus_slot_height_resolver-keypair.json)
PPR=$(solana-keygen pubkey target/deploy/janus_pyth_price_resolver-keypair.json)
MF=$(solana-keygen pubkey target/deploy/janus_market_factory-keypair.json)

echo "conditional-tokens:      $CT"
echo "lmsr-market:             $LM"
echo "lmsr-true-market:        ${LMT:-(no keypair, skipping)}"
echo "slot-height-resolver:    $SHR"
echo "pyth-price-resolver:     $PPR"
echo "market-factory:          $MF"

# Replace `declare_id!("...")` occurrences in every program's lib.rs.
update_declare_id() {
  local file=$1
  local pubkey=$2
  python3 - <<PY
import re, sys
p, k = "$file", "$pubkey"
with open(p) as f: s = f.read()
new = re.sub(r'declare_id!\("[^"]+"\)', f'declare_id!("{k}")', s, count=1)
if new != s:
    open(p, "w").write(new)
    print(f"  updated declare_id in {p}")
else:
    print(f"  no change in {p}")
PY
}

update_declare_id programs/conditional-tokens/src/lib.rs   "$CT"
update_declare_id programs/lmsr-market/src/lib.rs           "$LM"
if [[ -n "$LMT" ]]; then
  update_declare_id programs/lmsr-true-market/src/lib.rs    "$LMT"
fi
update_declare_id programs/slot-height-resolver/src/lib.rs  "$SHR"
update_declare_id programs/pyth-price-resolver/src/lib.rs   "$PPR"
update_declare_id programs/market-factory/src/lib.rs        "$MF"

# market-factory references CT and LMSR IDs as constants; keep them in sync.
python3 - <<PY
import re
p = "programs/market-factory/src/lib.rs"
ct, lm = "$CT", "$LM"
s = open(p).read()
s = re.sub(r'(CONDITIONAL_TOKENS_ID: Pubkey =\s*\n\s*pinocchio_pubkey::pubkey!\(")[^"]+(")', rf'\g<1>{ct}\g<2>', s)
s = re.sub(r'(LMSR_MARKET_ID: Pubkey =\s*\n\s*pinocchio_pubkey::pubkey!\(")[^"]+(")', rf'\g<1>{lm}\g<2>', s)
open(p, "w").write(s)
print("  updated cross-program IDs in market-factory")
PY

# lmsr-market's WithdrawPoolTokens references conditional-tokens.
python3 - <<PY
import re
p = "programs/lmsr-market/src/processor.rs"
ct = "$CT"
s = open(p).read()
s = re.sub(r'(CONDITIONAL_TOKENS_ID: Pubkey =\s*\n\s*pinocchio_pubkey::pubkey!\(")[^"]+(")', rf'\g<1>{ct}\g<2>', s)
open(p, "w").write(s)
print("  updated CT ref in lmsr-market processor")
PY

# SDK constants.ts
python3 - <<PY
p = "sdk/src/constants.ts"
mapping = {
    "CONDITIONAL_TOKENS_PROGRAM_ID": "$CT",
    "LMSR_MARKET_PROGRAM_ID":        "$LM",
    "SLOT_HEIGHT_RESOLVER_PROGRAM_ID":"$SHR",
    "PYTH_PRICE_RESOLVER_PROGRAM_ID":"$PPR",
    "MARKET_FACTORY_PROGRAM_ID":     "$MF",
}
import re
s = open(p).read()
for name, pk in mapping.items():
    s = re.sub(rf'(export const {name} = new PublicKey\(\s*")[^"]+(",\s*\);)', rf'\g<1>{pk}\g<2>', s)
open(p, "w").write(s)
print("  updated SDK constants")
PY

# tests/src/lib.rs ids module
python3 - <<PY
import re
p = "tests/src/lib.rs"
mapping = {
    "conditional_tokens":      "$CT",
    "lmsr_market":             "$LM",
    "lmsr_true_market":        "${LMT:-}",
    "slot_height_resolver":    "$SHR",
    "pyth_price_resolver":     "$PPR",
    "market_factory":          "$MF",
}
mapping = {k: v for k, v in mapping.items() if v}
s = open(p).read()
for name, pk in mapping.items():
    s = re.sub(
        rf'(pub fn {name}\(\) -> Pubkey \{{\s*\n\s*")[^"]+(")',
        rf'\g<1>{pk}\g<2>',
        s,
    )
open(p, "w").write(s)
print("  updated tests/src/lib.rs ids")
PY

echo "Sync complete. Rebuild: cargo build-sbf  &&  cd sdk && pnpm build"
