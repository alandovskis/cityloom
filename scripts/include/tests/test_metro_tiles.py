"""Tests of scripts/metro_tiles.py: `just test-scripts`. osmium is a fake program that writes small files."""

import argparse
import hashlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import metro_tiles  # noqa: E402
from include.progress import Progress  # noqa: E402
from support import fake_tools  # noqa: E402

# What osmium does here: `extract --config` writes each tile of the config (the tiles of column 0 are big enough to be
# kept, the others only a header's worth); any other command writes the file after -o. FAKE_OSMIUM_FAIL makes it fail.
OSMIUM = """
import json, os
if os.environ.get("FAKE_OSMIUM_FAIL"):
    print("reading the extract", file=sys.stderr)
    print("osmium: the thing broke", file=sys.stderr)
    sys.exit(1)
if "--config" in args:
    config = json.load(open(args[args.index("--config") + 1]))
    for extract in config["extracts"]:
        size = 500 if extract["output"].startswith("0_") else 100
        open(os.path.join(config["directory"], extract["output"]), "wb").write(b"t" * size)
else:
    open(args[args.index("-o") + 1], "wb").write(b"data")
"""


class Scratch(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.root = Path(self.dir.name)
        patch = mock.patch.object(metro_tiles, "OUT", self.root / "metro")
        patch.start()
        self.addCleanup(patch.stop)


class Arguments(unittest.TestCase):
    def test_batch_size_is_between_1_and_the_most_osmium_allows(self):
        self.assertEqual(metro_tiles.batch_size("1"), 1)
        self.assertEqual(metro_tiles.batch_size("500"), 500)
        for bad in ("0", "501", "-3"):
            with self.subTest(bad), self.assertRaises(argparse.ArgumentTypeError):
                metro_tiles.batch_size(bad)
        with self.assertRaises(ValueError):  # argparse turns this into its usage message
            metro_tiles.batch_size("many")

    def test_the_steps_are_six_and_the_tiles_one_names_the_grid(self):
        self.assertEqual([s.key for s in metro_tiles.STEPS], ["area", "highways", "tracks", "merge", "tiles", "index"])
        self.assertEqual(len({s.key for s in metro_tiles.STEPS}), 6)
        self.assertEqual(metro_tiles.STEPS[4].subject, "cutting 1,734 tiles")

    def test_main_refuses_a_bad_batch(self):
        with mock.patch.object(sys, "argv", ["metro_tiles.py", "q.pbf", "--batch", "0"]):
            with mock.patch.object(sys, "stderr", io.StringIO()), self.assertRaises(SystemExit) as caught:
                metro_tiles.main()
        self.assertEqual(caught.exception.code, 2)

    def test_main_hands_run_absolute_paths_and_the_batch(self):
        argv = ["metro_tiles.py", "q.pbf", "work", "--batch", "7"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(sys, "stderr", io.StringIO()):
            with mock.patch.object(metro_tiles, "run") as run:
                metro_tiles.main()
        _, source, work, batch = run.call_args.args
        self.assertEqual((source, work, batch), (Path("q.pbf").resolve(), Path("work").resolve(), 7))


class WriteIndex(Scratch):
    def test_it_names_the_grid_the_tiles_and_their_checksums(self):
        metro_tiles.OUT.mkdir()
        a, b = metro_tiles.OUT / "0_1.osm.pbf", metro_tiles.OUT / "2_3.osm.pbf"
        a.write_bytes(b"first")
        b.write_bytes(b"second")
        metro_tiles.write_index([a, b])
        text = (metro_tiles.OUT / "index.json").read_text()
        self.assertTrue(text.endswith("\n"))
        self.assertNotIn(" ", text)
        index = json.loads(text)
        self.assertEqual(
            (index["lon0"], index["lat0"], index["dlon"], index["dlat"]),
            (metro_tiles.LON0, metro_tiles.LAT0, metro_tiles.DLON, metro_tiles.DLAT),
        )
        self.assertEqual(index["tiles"], ["0_1", "2_3"])
        self.assertEqual(index["checksums"], {"0_1": hashlib.sha256(b"first").hexdigest()[:16],
                                              "2_3": hashlib.sha256(b"second").hexdigest()[:16]})


class Run(Scratch):
    def progress(self, out):
        return Progress(metro_tiles.STEPS, self.root / "logs", self.root, out=out, env={})

    def test_it_cuts_keeps_the_tiles_that_are_not_empty_and_writes_the_index(self):
        source, work = self.root / "q.pbf", self.root / "work"
        source.write_bytes(b"q")
        metro_tiles.OUT.mkdir()
        (metro_tiles.OUT / "old.osm.pbf").write_bytes(b"left from before")
        out = io.StringIO()
        with fake_tools(self.root / "bin", osmium=OSMIUM), self.progress(out) as p:
            metro_tiles.run(p, source, work, 500)
        tiles = sorted(q.name for q in metro_tiles.OUT.glob("*.osm.pbf"))
        self.assertEqual(tiles, [f"0_{y}.osm.pbf" for y in sorted(range(metro_tiles.NY), key=str)])
        index = json.loads((metro_tiles.OUT / "index.json").read_text())
        self.assertEqual(index["tiles"], [t.removesuffix(".osm.pbf") for t in tiles])
        text = out.getvalue()
        self.assertIn("RUNS tiles cutting 1,734 tiles batch 4 of 4", text)
        self.assertIn(f"PASS tiles {metro_tiles.NY} tiles 0.0 MB", text)
        self.assertIn("Steps: 6 passed, 6 total", text)
        self.assertTrue((work / "tiles.json").exists())
        log = (self.root / "logs" / "tracks.log").read_text()
        self.assertEqual(log.count("$ osmium tags-filter"), 2)

    def test_a_smaller_batch_runs_osmium_more_times(self):
        source = self.root / "q.pbf"
        source.write_bytes(b"q")
        with fake_tools(self.root / "bin", osmium=OSMIUM), self.progress(io.StringIO()) as p:
            metro_tiles.run(p, source, self.root / "work", 400)
        self.assertEqual((self.root / "logs" / "tiles.log").read_text().count("$ osmium extract --config"), 5)

    def test_a_missing_extract_says_where_to_get_it(self):
        out = io.StringIO()
        with self.assertRaises(SystemExit), self.progress(out) as p:
            metro_tiles.run(p, self.root / "none.pbf", self.root / "work", 500)
        text = out.getvalue()
        self.assertIn("FAIL roads metro area", text)
        self.assertIn("none.pbf does not exist. Download the Quebec extract first", text)
        self.assertIn("curl -L -o quebec-latest.osm.pbf", text)

    def test_a_failing_osmium_shows_the_end_of_its_log(self):
        source = self.root / "q.pbf"
        source.write_bytes(b"q")
        out = io.StringIO()
        with fake_tools(self.root / "bin", osmium=OSMIUM), mock.patch.dict("os.environ", {"FAKE_OSMIUM_FAIL": "1"}):
            with self.assertRaises(SystemExit) as caught, self.progress(out) as p:
                metro_tiles.run(p, source, self.root / "work", 500)
        self.assertEqual(caught.exception.code, 1)
        lines = out.getvalue().splitlines()
        failed = lines.index("FAIL roads metro area")
        shown = lines[failed + 1 : failed + 6]
        self.assertEqual(shown[0], " \u2023 osmium exited with status 1")
        self.assertTrue(shown[1].startswith(" \u2023 $ osmium extract -b "))  # the log begins with the command
        self.assertEqual(
            shown[2:],
            [" \u2023 reading the extract", " \u2023 osmium: the thing broke", " \u2023 full log: logs/area.log"],
        )
        self.assertIn("Steps: 1 failed, 6 total", lines)
        self.assertFalse((metro_tiles.OUT / "index.json").exists())


if __name__ == "__main__":
    unittest.main()
