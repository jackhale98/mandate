#!/usr/bin/env bash
# Re-record every recorded value that summarises a simulated world, in the
# right order, and say what moved.
#
#   scripts/repin.sh                 re-record, confirm the tests pass, print old -> new
#   scripts/repin.sh --check         change nothing; list what is stale; exit 1 if any
#   scripts/repin.sh --allow-dirty   re-record on a tree with uncommitted changes
#   scripts/repin.sh --wasm          also run the web pin test (needs a fresh scripts/build-wasm.sh)
#   scripts/repin.sh --impact        first refit the impact table (slow; off by default)
#
# Use it only when a commit changes outcomes (a balance fix, a new system) or
# the state's shape on purpose, and say why in the commit message
# (docs/development/TEST_STRATEGY.md, "Pins and re-recording"). It never edits
# an assertion: only the recorded values below.
#
# What it records, in this order:
#   0. content/base/impact.jsonc             only with --impact: mandate-cli lab impact calibrate
#        (DEC-111; a release build). Fitted to the simulation's numbers, so it is refitted when
#        a mechanism or the economy changes (the end-of-M6 tuning pass). By default the run the
#        shipped table records, again (--refit); MANDATE_IMPACT_ARGS replaces that, for example
#        MANDATE_IMPACT_ARGS="--seeds 16" for the full fit (hours). It comes first because a law
#        keeps the estimate it passed with: the pins below hold what the table gives.
#        `mandate-cli audit estimate` then judges the new table on other seeds.
#   1. tests/seeds.json                      mandate-cli seeds refresh
#   2. the state-shape fingerprint           crates/mandate-save/tests/saves.rs
#   3. the town geography pin                crates/mandate-wasm/tests/town.rs and
#                                            web/src/hub/fixtures/engineViews.test.ts
#   4. recorded fixtures (MANDATE_RECORD_FIXTURES / MANDATE_UPDATE_FIXTURES)
#        crates/mandate-save/tests/fixtures/v18_campaign_continued.mndt (git add -f)
#        web/src/components/asks/asks.fixture.json
#        web/src/hub/layoutV2/registry/base.json
#   5. the spine proof                       PINS and BEFORE_ARCS in
#                                            crates/mandate-cli/src/lab/proof.rs
#                                            (SPINE_PROOF_PRINT, SPINE_PROOF_NO_ARCS=print)
# Not handled: the pixel-art goldens (tools/pixelkit, UPDATE_GOLDENS=1; art, not
# a world) and web/src/hub/fixtures/towns-v2.json (DUMP_GEO=1; nothing asserts it
# against the engine). Determinism and WASM parity hashes are computed on the
# run, not pinned.
# Honours CARGO_TARGET_DIR, CARGO_BUILD_JOBS and MANDATE_LAB_THREADS.
set -uo pipefail
cd "$(dirname "$0")/.."

check=0; allow_dirty=0; wasm=0; impact=0
for a in "$@"; do
  case "$a" in
    --check) check=1 ;;
    --allow-dirty) allow_dirty=1 ;;
    --wasm) wasm=1 ;;
    --impact) impact=1 ;;
    -h|--help) sed -n '2,/^set -uo/p' "$0" | sed '$d;s/^# \{0,1\}//'; exit 0 ;;
    *) echo "usage: scripts/repin.sh [--check] [--allow-dirty] [--wasm] [--impact]" >&2; exit 2 ;;
  esac
done

PROOF=crates/mandate-cli/src/lab/proof.rs
SAVES=crates/mandate-save/tests/saves.rs
TOWN=crates/mandate-wasm/tests/town.rs
VIEWS=web/src/hub/fixtures/engineViews.test.ts
SEEDS=tests/seeds.json
IMPACT=content/base/impact.jsonc
FIXTURES=(
  crates/mandate-save/tests/fixtures/v18_campaign_continued.mndt
  web/src/components/asks/asks.fixture.json
  web/src/hub/layoutV2/registry/base.json
)
FILES=("$PROOF" "$SAVES" "$TOWN" "$VIEWS" "$SEEDS" "$IMPACT" "${FIXTURES[@]}")

if ((!check && !allow_dirty)) && [[ -n "$(git status --porcelain)" ]]; then
  echo "repin: the tree has uncommitted changes; commit or stash them (or pass --allow-dirty)." >&2
  git status --short >&2
  exit 2
fi

TMP="$(mktemp -d)"
restore_originals() {
  local f
  for f in "${FILES[@]}"; do [[ -f "$TMP/orig/$f" ]] && cp "$TMP/orig/$f" "$f"; done
}
cleanup() { ((check)) && [[ -d "$TMP/orig" ]] && restore_originals; rm -rf "$TMP"; }
trap cleanup EXIT
for f in "${FILES[@]}"; do mkdir -p "$TMP/orig/$(dirname "$f")"; cp "$f" "$TMP/orig/$f"; done

PY=$(cat <<'PYEOF'
import hashlib, json, re, sys

PROOF = "crates/mandate-cli/src/lab/proof.rs"
SAVES = "crates/mandate-save/tests/saves.rs"
TOWN = "crates/mandate-wasm/tests/town.rs"
VIEWS = "web/src/hub/fixtures/engineViews.test.ts"
SEEDS = "tests/seeds.json"
FIXTURES = [
    "crates/mandate-save/tests/fixtures/v18_campaign_continued.mndt",
    "web/src/components/asks/asks.fixture.json",
    "web/src/hub/layoutV2/registry/base.json",
]
ENTRY = re.compile(r'\(\s*(\d+),\s*"([^"]+)",\s*(\d+),\s*"([0-9a-f]+)",\s*"([0-9a-f]+)",?\s*\),(\s*)// (\d+) events')
SHAPE = re.compile(r'(const EXPECTED: \(u16, &str\) = \(\d+, ")([0-9a-f]+)(")')
GEO_RS = re.compile(r'(const PINNED_GEOGRAPHY: \(&str, u64, &str\) = \("[a-z]+", \d+, ")([0-9a-f]+)(")')
GEO_TS = re.compile(r'(const PINNED = \{[^}]*hash: ")([0-9a-f]+)(")')

def read(p):
    return open(p, encoding="utf8").read()

def table_block(src, name):
    i = src.index("const %s:" % name)
    j = src.index("];", i)
    return i, j

def proof_pins(src, name):
    i, j = table_block(src, name)
    return {"%s:%s:%s:%s" % (name, m.group(1), m.group(2), m.group(3)): "%s/%s (%s events)" % (m.group(4), m.group(5), m.group(7))
            for m in ENTRY.finditer(src[i:j])}

def snapshot():
    out = {}
    src = read(PROOF)
    out.update(proof_pins(src, "PINS"))
    out.update(proof_pins(src, "BEFORE_ARCS"))
    out["shape fingerprint (saves.rs)"] = SHAPE.search(read(SAVES)).group(2)
    out["geography (town.rs)"] = GEO_RS.search(read(TOWN)).group(2)
    out["geography (engineViews.test.ts)"] = GEO_TS.search(read(VIEWS)).group(2)
    for f in FIXTURES:
        out["fixture " + f] = hashlib.sha1(open(f, "rb").read()).hexdigest()[:12]
    for k, v in json.loads(read(SEEDS)).items():
        out["seed " + k] = str(v["seed"])
    out["impact table (impact.jsonc)"] = hashlib.sha1(open("content/base/impact.jsonc", "rb").read()).hexdigest()[:12]
    return out

def replace_one(path, rx, value):
    s = read(path)
    if not rx.search(s):
        sys.exit("repin: cannot find the pin in " + path)
    open(path, "w", encoding="utf8").write(rx.sub(lambda m: m.group(1) + value + m.group(3), s, count=1))

def apply_proof(name, log):
    new = {(m.group(1), m.group(2), m.group(3)): m for m in ENTRY.finditer(read(log))}
    src = read(PROOF)
    i, j = table_block(src, name)
    seen = set()
    def swap(m):
        key = (m.group(1), m.group(2), m.group(3))
        if key not in new:
            sys.exit("repin: no recorded value for %s %s (the proof's game list changed?)" % (name, key))
        n = new[key]
        seen.add(key)
        whole = m.group(0)
        whole = whole.replace('"%s"' % m.group(4), '"%s"' % n.group(4), 1)
        whole = whole.replace('"%s"' % m.group(5), '"%s"' % n.group(5), 1)
        return whole[: whole.rindex("// ")] + "// %s events" % n.group(7)
    body = ENTRY.sub(swap, src[i:j])
    if name == "PINS" and seen != set(new):
        sys.exit("repin: the proof printed games that PINS does not list: %s" % sorted(set(new) - seen))
    open(PROOF, "w", encoding="utf8").write(src[:i] + body + src[j:])

def table(old_path, new_path):
    old, new = json.load(open(old_path)), json.load(open(new_path))
    rows = [(k, old.get(k, "-"), new.get(k, "-")) for k in sorted(set(old) | set(new)) if old.get(k) != new.get(k)]
    if not rows:
        print("no pin moved")
        return 0
    w0 = max(len(r[0]) for r in rows); w1 = max(len(r[1]) for r in rows)
    print("%-*s  %-*s  %s" % (w0, "pin", w1, "old", "new"))
    for r in rows:
        print("%-*s  %-*s  %s" % (w0, r[0], w1, r[1], r[2]))
    return len(rows)

cmd = sys.argv[1]
if cmd == "snapshot":
    json.dump(snapshot(), open(sys.argv[2], "w"), indent=1)
elif cmd == "proof":
    apply_proof(sys.argv[2], sys.argv[3])
elif cmd == "shape":
    replace_one(SAVES, SHAPE, sys.argv[2])
elif cmd == "geography":
    replace_one(TOWN, GEO_RS, sys.argv[2])
    replace_one(VIEWS, GEO_TS, sys.argv[2])
elif cmd == "table":
    sys.exit(1 if table(sys.argv[2], sys.argv[3]) else 0)
elif cmd == "value":
    # value <log> <name>: the last "REPIN <name> <value>" line.
    hits = re.findall(r"^REPIN %s ([0-9a-f]+)$" % re.escape(sys.argv[3]), read(sys.argv[2]), re.M)
    if not hits:
        sys.exit("repin: no REPIN %s line in %s" % (sys.argv[3], sys.argv[2]))
    print(hits[-1])
PYEOF
)
py() { python3 -c "$PY" "$@"; }

# run <name> <command...>: a recorder or a confirming test, logged; stop if it fails.
run() {
  local name="$1"; shift
  echo "... $name"
  if ! "$@" >"$TMP/$name.log" 2>&1; then
    echo "repin: '$name' failed (not a stale pin: look at it):" >&2
    tail -n 30 "$TMP/$name.log" >&2
    exit 2
  fi
}

py snapshot "$TMP/old.json" || exit 2

# 0. The impact table, when asked for: everything below is recorded with it.
if ((impact)); then
  # shellcheck disable=SC2086
  run impact cargo run -q --release -p mandate-cli -- lab impact calibrate ${MANDATE_IMPACT_ARGS:---refit}
fi

# 1. The seed cache first: the tests below look their seeds up in it.
run seeds cargo run -q -p mandate-cli -- seeds refresh

# 2. The state-shape fingerprint.
MANDATE_RECORD_PINS=1 run shape cargo test -q -p mandate-save --test saves state_shape_matches_format_version -- --nocapture
py shape "$(py value "$TMP/shape.log" shape)" || exit 2

# 3. The town geography pin, native and web together.
MANDATE_RECORD_PINS=1 run geography cargo test -q -p mandate-wasm --test town geography_is_static -- --nocapture
py geography "$(py value "$TMP/geography.log" geography)" || exit 2

# 4. Recorded fixtures.
MANDATE_RECORD_FIXTURES=1 run fixture-v18 cargo test -q -p mandate-save --test constitution_posts v18_save_continues
MANDATE_UPDATE_FIXTURES=1 run fixture-asks cargo test -q -p mandate-wasm --test asks the_web_fixture
MANDATE_UPDATE_FIXTURES=1 run fixture-layout cargo test -q -p mandate-protocol layout

# 5. The spine proof: PINS, then BEFORE_ARCS (the no-arc digests of the games already listed).
SPINE_PROOF_PRINT=1 run proof cargo test -q -p mandate-cli the_spine_changes_no_outcome -- --nocapture
SPINE_PROOF_NO_ARCS=print run proof-no-arcs cargo test -q -p mandate-cli with_no_arc_content_every_pin_holds -- --nocapture
py proof PINS "$TMP/proof.log" || exit 2
py proof BEFORE_ARCS "$TMP/proof-no-arcs.log" || exit 2
if ((!check)); then cargo fmt --all >/dev/null 2>&1; fi

py snapshot "$TMP/new.json" || exit 2

if ((check)); then
  restore_originals
  echo
  if py table "$TMP/old.json" "$TMP/new.json"; then
    echo "repin --check: every pin is current"
    exit 0
  fi
  echo
  echo "repin --check: the pins above are stale. If this commit changes outcomes or the state's shape on purpose, run scripts/repin.sh and say why in the commit message."
  exit 1
fi

git add -f "${FIXTURES[0]}" >/dev/null 2>&1

# Confirm: the tests that read these pins, run as they are.
echo
echo "confirming:"
run confirm-seeds cargo run -q -p mandate-cli -- seeds check
run confirm-seeds-unit cargo test -q -p mandate-seeds
run confirm-shape cargo test -q -p mandate-save --test saves state_shape_matches_format_version
run confirm-fixture cargo test -q -p mandate-save --test constitution_posts v18_save_continues
run confirm-geography cargo test -q -p mandate-wasm --test town
run confirm-asks cargo test -q -p mandate-wasm --test asks
run confirm-layout cargo test -q -p mandate-protocol layout
run confirm-proof cargo test -q -p mandate-cli -- the_spine_changes_no_outcome with_no_arc_content_every_pin_holds
if ((wasm)); then
  run confirm-web bash -c 'cd web && npx vitest run src/hub/fixtures/engineViews.test.ts'
else
  echo "(web pin test not run: rebuild with scripts/build-wasm.sh, then cd web && npx vitest run src/hub/fixtures/engineViews.test.ts, or pass --wasm)"
fi

echo
py table "$TMP/old.json" "$TMP/new.json"
exit 0
