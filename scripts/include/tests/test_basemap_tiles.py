"""Tests of scripts/basemap_tiles.py: `just test-scripts`. Java is a fake program; the jar comes from a local server."""

import argparse
import http.server
import io
import sys
import tempfile
import unittest
import urllib.request
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import basemap_tiles  # noqa: E402
from include.progress import Progress  # noqa: E402
from support import fake_tools, serve  # noqa: E402

JAR = b"J" * 3000

# Writes the file named by --output, and the arguments it was given to a file beside it; FAKE_JAVA_FAIL makes it fail.
JAVA = """
import os
if os.environ.get("FAKE_JAVA_FAIL"):
    print("java.lang.OutOfMemoryError", file=sys.stderr)
    sys.exit(1)
output = [a for a in args if a.startswith("--output=")][0].removeprefix("--output=")
open(output, "wb").write(b"tiles" * 1000)
open(output + ".args", "w").write("\\n".join(args))
"""


def releases():
    """A server whose /latest redirects to a release's address, which redirects to a signed address of the jar."""

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_GET(self):
            if self.path == "/latest":
                self.send_response(302)
                self.send_header("Location", "/releases/download/v1.2.3/planetiler.jar")
            elif self.path == "/releases/download/v1.2.3/planetiler.jar":
                self.send_response(302)
                self.send_header("Location", "/signed?token=abc")
            else:
                self.send_response(200)
                self.send_header("Content-Length", str(len(JAR)))
            self.end_headers()
            if self.path.startswith("/signed"):
                self.wfile.write(JAR)

    return Handler


class Scratch(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.root = Path(self.dir.name)
        tools = self.root / ".tools"
        for name, value in (
            ("TOOLS", tools),
            ("JAR", tools / "planetiler.jar"),
            ("JAR_SOURCE", tools / "planetiler.jar.source"),
            ("ROOT", self.root),
        ):
            patch = mock.patch.object(basemap_tiles, name, value)
            patch.start()
            self.addCleanup(patch.stop)

    def progress(self, out):
        return Progress(basemap_tiles.STEPS, self.root / "logs", self.root, out=out, env={})


class Arguments(unittest.TestCase):
    def test_bounds_are_four_numbers_and_memory_a_size(self):
        bounds = basemap_tiles.matching(basemap_tiles.BOUNDS_FORMAT, "bounds")
        self.assertEqual(bounds("-74.40,45.20,-73.09,45.81"), "-74.40,45.20,-73.09,45.81")
        self.assertEqual(bounds("1,2,3,4"), "1,2,3,4")
        memory = basemap_tiles.matching(basemap_tiles.MEMORY_FORMAT, "a size")
        self.assertEqual((memory("4g"), memory("512M")), ("4g", "512M"))
        for check, bad in ((bounds, "1,2,3"), (bounds, "1,2,3,4 --evil"), (bounds, "a,b,c,d"), (memory, "4"),
                           (memory, "4g -jar x"), (memory, "")):
            with self.subTest(bad), self.assertRaises(argparse.ArgumentTypeError) as caught:
                check(bad)
            self.assertIn(repr(bad), str(caught.exception))

    def test_the_steps_are_the_jar_and_the_map(self):
        self.assertEqual([s.key for s in basemap_tiles.STEPS], ["planetiler", "basemap"])
        self.assertEqual([s.subject for s in basemap_tiles.STEPS], ["planetiler.jar", "montreal.pmtiles"])

    def test_main_hands_run_the_settings(self):
        argv = ["basemap_tiles.py", "q.pbf", "out.pmtiles", "--minzoom", "2", "--maxzoom", "9", "--memory", "2g"]
        with mock.patch.object(sys, "argv", argv), mock.patch.object(sys, "stderr", io.StringIO()):
            with mock.patch.object(basemap_tiles, "run") as run:
                basemap_tiles.main()
        _, extract, output, bounds, minzoom, maxzoom, memory = run.call_args.args
        self.assertEqual((extract, output), (Path("q.pbf").resolve(), Path("out.pmtiles").resolve()))
        self.assertEqual((bounds, minzoom, maxzoom, memory), (basemap_tiles.METRO_BOUNDS, 2, 9, "2g"))

    def test_main_refuses_bad_bounds(self):
        with mock.patch.object(sys, "argv", ["basemap_tiles.py", "q.pbf", "--bounds", "1,2"]):
            with mock.patch.object(sys, "stderr", io.StringIO()), self.assertRaises(SystemExit) as caught:
                basemap_tiles.main()
        self.assertEqual(caught.exception.code, 2)


class Release(unittest.TestCase):
    def test_it_keeps_the_address_that_names_the_release_not_the_signed_one(self):
        release = basemap_tiles._Release()
        with serve(releases()) as url:
            with urllib.request.build_opener(release).open(url + "/latest") as response:
                self.assertEqual(response.read(), JAR)
        self.assertEqual(release.release, url + "/releases/download/v1.2.3/planetiler.jar")


class Planetiler(Scratch):
    def fetch(self):
        out = io.StringIO()
        with self.progress(out) as p, p.step("planetiler") as step, serve(releases()) as url:
            with mock.patch.object(basemap_tiles, "JAR_URL", url + "/latest"):
                jar = basemap_tiles.planetiler(p, step)
        return jar, step, url

    def test_a_kept_jar_is_not_downloaded(self):
        basemap_tiles.JAR.parent.mkdir()
        basemap_tiles.JAR.write_bytes(b"jar")
        with self.progress(io.StringIO()) as p, p.step("planetiler") as step:
            with mock.patch.object(urllib.request, "build_opener") as opener:
                self.assertEqual(basemap_tiles.planetiler(p, step), basemap_tiles.JAR)
        opener.assert_not_called()
        self.assertEqual(step.detail, "kept")

    def test_it_downloads_the_jar_whole_and_says_which_release(self):
        jar, step, url = self.fetch()
        self.assertEqual(jar.read_bytes(), JAR)
        self.assertFalse(jar.with_name("planetiler.jar.part").exists())
        self.assertEqual(basemap_tiles.JAR_SOURCE.read_text(), url + "/releases/download/v1.2.3/planetiler.jar\n")
        self.assertEqual(step.detail, "0 MB")

    def test_a_failed_download_leaves_no_part_file_and_no_jar(self):
        class Broken(releases()):
            def do_GET(self):
                self.send_error(500)

        with serve(Broken) as url, self.progress(io.StringIO()) as p, p.step("planetiler") as step:
            with mock.patch.object(basemap_tiles, "JAR_URL", url + "/latest"), self.assertRaises(Exception) as caught:
                basemap_tiles.planetiler(p, step)
        caught.exception.close()
        self.assertEqual(list(basemap_tiles.TOOLS.iterdir()), [])


class Run(Scratch):
    def setUp(self):
        super().setUp()
        self.extract, self.output = self.root / "q.pbf", self.root / "out" / "montreal.pmtiles"
        self.extract.write_bytes(b"q")
        basemap_tiles.JAR.parent.mkdir()
        basemap_tiles.JAR.write_bytes(b"jar")

    def run_it(self, out):
        with self.progress(out) as p:
            basemap_tiles.run(p, self.extract, self.output, "1,2,3,4", 1, 8, "3g")

    def test_it_runs_java_with_the_settings_and_reports_the_size(self):
        out = io.StringIO()
        with fake_tools(self.root / "bin", java=JAVA):
            self.run_it(out)
        args = Path(str(self.output) + ".args").read_text().splitlines()
        self.assertEqual(args[:3], ["-Xmx3g", "-jar", str(basemap_tiles.JAR)])
        for expected in (f"--osm-path={self.extract}", "--bounds=1,2,3,4", "--minzoom=1", "--maxzoom=8", "--force"):
            self.assertIn(expected, args)
        text = out.getvalue()
        self.assertIn("PASS basemap planetiler.jar kept", text)
        self.assertIn("PASS basemap montreal.pmtiles 0.0 MB", text)
        self.assertIn("Steps: 2 passed, 2 total", text)

    def test_a_missing_extract_says_where_to_get_it(self):
        self.extract.unlink()
        out = io.StringIO()
        with self.assertRaises(SystemExit):
            self.run_it(out)
        self.assertIn(f"{self.extract} is not there: get it from https://download.geofabrik.de", out.getvalue())

    def test_a_failing_java_shows_the_end_of_its_log(self):
        out = io.StringIO()
        with fake_tools(self.root / "bin", java=JAVA), mock.patch.dict("os.environ", {"FAKE_JAVA_FAIL": "1"}):
            with self.assertRaises(SystemExit) as caught:
                self.run_it(out)
        self.assertEqual(caught.exception.code, 1)
        lines = out.getvalue().splitlines()
        failed = lines.index("FAIL basemap montreal.pmtiles")
        self.assertEqual(lines[failed + 1], " ‣ java exited with status 1")
        self.assertEqual(lines[failed + 3 : failed + 5], [" ‣ java.lang.OutOfMemoryError", " ‣ full log: logs/basemap.log"])
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
