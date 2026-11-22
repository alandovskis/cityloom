#!/bin/sh
# Captures the evidence for CLM-31, every step from this one script. Run from the worktree, on a clean tree at the
# head to evidence, with DATABASE_URL set (the server's tests need a Postgres: `just db`).
# Writes 01..04 and copies 05-measure.py and this script (06-capture-all.sh) into the evidence folder.
set -u
S=/private/tmp/claude-501/-Users-alex-src-cityloom-main/40f831c0-2185-4631-8cf3-8f332b786f47/scratchpad
E=$S/evidence-clm31
MAIN=/Users/alex/src/cityloom/main
rm -rf "$E"
mkdir -p "$E"
HEAD=$(git rev-parse HEAD)

# 1. The new tests against the importer code as on main: the carry pass removed, the fold_nodes rule reverted.
git checkout origin/main -- crates/citymodel_builder/src/importer/mod.rs
sed -i '' 's/            if n.control == crate::importer::Control::None || g.control == crate::importer::Control::Signals {/            if n.control == crate::importer::Control::None {/' crates/citymodel_builder/src/importer/merge.rs
{
  echo "\$ git rev-parse HEAD"
  echo "$HEAD"
  echo "\$ git checkout origin/main -- crates/citymodel_builder/src/importer/mod.rs   # main's importer: no carry pass"
  echo "\$ sed ... merge.rs   # fold_nodes takes the other node's control only when it has none, as on main"
  git diff --stat HEAD
  echo "\$ cargo test -p citymodel_builder --lib importer::merge::tests::a_junction_is_signalled"
  cargo test -q -p citymodel_builder --lib importer::merge::tests::a_junction_is_signalled 2>&1 | grep -E "^test |panicked|left|right|test result"
  echo
  echo "\$ cargo test -p citymodel_builder --test importer   # failures listed by name, then the totals"
  cargo test -q -p citymodel_builder --test importer 2>&1 | grep -E "^    import::|left:|right:|test result"
} > "$E/01-failing-on-main.txt" 2>&1
git checkout HEAD -- crates/citymodel_builder/src/importer/mod.rs crates/citymodel_builder/src/importer/merge.rs
test -z "$(git status --short)" || { echo "the tree is not clean after step 1" >&2; exit 1; }

# 2. The same tests at the head.
{
  echo "\$ git rev-parse HEAD"
  echo "$HEAD"
  echo "\$ cargo test -p citymodel_builder --lib importer::merge"
  cargo test -p citymodel_builder --lib importer::merge 2>&1 | grep -E "^test |test result"
  echo
  echo "\$ cargo test -p citymodel_builder --test importer   # every importer test, the new ones among them"
  cargo test -p citymodel_builder --test importer 2>&1 | grep -E "^test |test result"
  echo
  echo "\$ cargo test -p cityloom_model importer::tests"
  cargo test -p cityloom_model importer::tests 2>&1 | grep -E "^test |test result"
} > "$E/02-passing-at-head.txt" 2>&1

# 3. Real tiles at the head from the same extract as the current tiles (main's importer), measured.
rm -rf "$S/tiles-head" "$S/tiles-work"
mkdir -p "$S/tiles-head" "$S/tiles-work"
{
  echo "\$ git rev-parse HEAD"
  echo "$HEAD"
  echo "\$ cargo run --release -p citymodel_builder -- tiles $MAIN/data/quebec-latest.osm.pbf tiles-work --out tiles-head   # the steps \`just prepare\` runs"
  cargo run -q --release -p citymodel_builder -- tiles "$MAIN/data/quebec-latest.osm.pbf" "$S/tiles-work" --out "$S/tiles-head" 2>&1 | grep -E "^PASS|^FAIL|^Steps|^Time"
  echo "\$ cmp tiles-work/roads.osm.pbf $MAIN/data/metro-work/roads.osm.pbf   # the roads the current tiles were made from"
  cmp "$S/tiles-work/roads.osm.pbf" "$MAIN/data/metro-work/roads.osm.pbf" && echo "identical"
  echo "\$ osmium cat tiles-work/roads.osm.pbf -f opl -o roads.opl"
  osmium cat "$S/tiles-work/roads.osm.pbf" -f opl -o "$S/opl/roads.opl" --overwrite && echo "(written)"
  echo "\$ python3 -I 05-measure.py roads.opl $MAIN/web/data/metro tiles-head   # before: main's importer (version 11); after: the head (version 12)"
  echo
  python3 -I "$S/measure.py" "$S/opl/roads.opl" "$MAIN/web/data/metro" "$S/tiles-head"
} > "$E/03-montreal-tiles-before-after.txt" 2>&1

# 3b. Why the junctions of 3+ roads that a road's signals reach stay signs (all signs before as well).
{
  echo "\$ python3 -I 08-seven.py roads.opl $MAIN/web/data/metro tiles-head   # each such signal: its target, the node it reaches before and after, its way"
  python3 -I "$S/seven.py" "$S/opl/roads.opl" "$MAIN/web/data/metro" "$S/tiles-head"
  echo
  echo "\$ python3 -I 09-at.py roads.opl <target nodes>   # the road ways at each target the tiles have no node for"
  python3 -I "$S/at.py" "$S/opl/roads.opl" 9994840111 11134143778 522276554 1764298494 10927072417 12053666619 12982440247 10998451695
  echo
  echo "\$ python3 -I 10-dist.py roads.opl <split>:<node ahead>   # length along one way"
  python3 -I "$S/dist.py" "$S/opl/roads.opl" 9994840111:8929187841 522276554:4667882258 10927072417:5098156688 12053666619:4463269465 10998451695:10998451696
  echo
  echo "\$ grep '^n11134143778 ' roads.opl   # the node between split 10927072417 and junction 5098156688"
  grep -E "^n11134143778 " "$S/opl/roads.opl"
  echo "\$ grep '^n9994840111 ' roads.opl   # split 9994840111 itself"
  grep -E "^n9994840111 " "$S/opl/roads.opl"
} > "$E/07-road-signal-junctions.txt" 2>&1

# 4. The whole workspace: host tests of every crate, then the wasm build, warnings as errors; and rustfmt.
{
  echo "\$ git rev-parse HEAD"
  echo "$HEAD"
  echo "\$ RUSTFLAGS=\"-D warnings\" just test   # DATABASE_URL set to the local test Postgres"
  RUSTFLAGS="-D warnings" just test > "$S/just-test-evidence.log" 2>&1
  echo "exit $?"
  grep -E "^test result" "$S/just-test-evidence.log"
  grep -E "^test result" "$S/just-test-evidence.log" | awk '{p+=$4; f+=$6} END {print "total: passed", p, "failed", f}'
  echo "\$ cargo fmt --all -- --check"
  cargo fmt --all -- --check && echo "exit 0, no output"
} > "$E/04-full-suite.txt" 2>&1

cp "$S/measure.py" "$E/05-measure.py"
cp "$0" "$E/06-capture-all.sh"
cp "$S/seven.py" "$E/08-seven.py"
cp "$S/at.py" "$E/09-at.py"
cp "$S/dist.py" "$E/10-dist.py"
ls -la "$E"
