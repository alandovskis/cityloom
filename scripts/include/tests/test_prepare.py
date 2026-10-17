"""Tests of scripts/prepare.py: `just test-scripts`. No network: the downloads are from a local server."""

import contextlib
import io
import sys
import tempfile
import unittest
import urllib.error
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import basemap_tiles  # noqa: E402
import metro_tiles  # noqa: E402
import prepare  # noqa: E402
from include.progress import Progress, StepError  # noqa: E402
from support import file_handler, serve  # noqa: E402

BODY = bytes(range(256)) * 40


class Scratch(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.root = Path(self.dir.name)


class Megabytes(unittest.TestCase):
    def test_with_and_without_a_total(self):
        self.assertEqual(prepare.megabytes(1_213_000_000, None), "1,213 MB")
        self.assertEqual(prepare.megabytes(5_000_000, 1_213_000_000), "5 of 1,213 MB")


class Download(Scratch):
    def fetch(self, ranges, part=b""):
        """Downloads BODY from a server that does or does not do ranges, from a `.part` of `part`; returns what the
        server was asked for (the Range headers) and what was said."""
        target = self.root / "sub" / "extract.pbf"
        target.parent.mkdir()
        if part:
            target.with_name("extract.pbf.part").write_bytes(part)
        seen, said = [], []
        with serve(file_handler(BODY, ranges, seen)) as url:
            prepare.download(url + "/x.pbf", target, lambda done, total: said.append((done, total)))
        return target, seen, said

    def test_a_fresh_download_is_renamed_once_whole(self):
        target, seen, said = self.fetch(ranges=True)
        self.assertEqual(target.read_bytes(), BODY)
        self.assertFalse(target.with_name("extract.pbf.part").exists())
        self.assertEqual(seen, [None])
        self.assertEqual(said[-1], (len(BODY), len(BODY)))

    def test_a_part_file_is_continued_when_the_server_does_ranges(self):
        target, seen, said = self.fetch(ranges=True, part=BODY[:1000])
        self.assertEqual(target.read_bytes(), BODY)
        self.assertEqual(seen, ["bytes=1000-"])
        self.assertEqual(said[-1], (len(BODY), len(BODY)))

    def test_a_part_file_is_started_again_when_the_server_does_not_do_ranges(self):
        target, seen, _ = self.fetch(ranges=False, part=b"stale bytes")
        self.assertEqual(target.read_bytes(), BODY)
        self.assertEqual(seen, ["bytes=11-"])

    def test_a_part_file_that_is_the_whole_file_is_just_renamed(self):
        target, seen, said = self.fetch(ranges=True, part=BODY)
        self.assertEqual(target.read_bytes(), BODY)
        self.assertEqual(seen, [f"bytes={len(BODY)}-"])
        self.assertEqual(said, [])

    def test_a_missing_file_is_an_error(self):
        class Gone(file_handler(BODY, False, [])):
            def do_GET(self):
                self.send_error(404)

        with serve(Gone) as url, self.assertRaises(urllib.error.HTTPError) as caught:
            prepare.open_from(url + "/x", 0)
        caught.exception.close()
        self.assertEqual(caught.exception.code, 404)

    def test_416_without_a_part_file_is_an_error(self):
        class Never(file_handler(BODY, False, [])):
            def do_GET(self):
                self.send_error(416)

        with serve(Never) as url, self.assertRaises(urllib.error.HTTPError) as caught:
            prepare.open_from(url + "/x", 0)
        caught.exception.close()

    def test_a_short_download_says_to_run_again_and_keeps_the_part(self):
        class Short(file_handler(BODY, False, [])):
            def do_GET(self):
                self.send_response(200)
                self.send_header("Content-Length", str(len(BODY)))
                self.end_headers()
                self.wfile.write(BODY[:500])
                self.close_connection = True

        target = self.root / "extract.pbf"
        with serve(Short) as url, self.assertRaises(StepError) as caught:
            prepare.download(url + "/x", target, lambda done, total: None)
        self.assertFalse(target.exists())
        self.assertEqual(caught.exception.lines, [f"{url}/x: got 500 bytes of {len(BODY)}; run again to continue"])
        self.assertEqual(target.with_name("extract.pbf.part").read_bytes(), BODY[:500])


class GetExtract(Scratch):
    def progress(self):
        return Progress([prepare.Spec("extract", "extract", "x")], self.root / "logs", self.root, out=io.StringIO(), env={})

    def test_a_kept_extract_is_not_downloaded(self):
        extract = self.root / "q.pbf"
        extract.write_bytes(b"x" * 2_000_000)
        with self.progress() as p, p.step("extract") as step, mock.patch.object(prepare, "download") as download:
            prepare.get_extract(p, step, extract, refresh=False)
        download.assert_not_called()
        self.assertEqual(step.detail, "2 MB, kept")

    def test_refresh_and_a_missing_extract_download(self):
        extract = self.root / "q.pbf"
        for refresh, present in ((True, True), (False, False)):
            extract.unlink(missing_ok=True)
            if present:
                extract.write_bytes(b"old")
            seen = []
            with serve(file_handler(BODY, True, seen)) as url, mock.patch.object(prepare, "URL", url + "/q"):
                with self.progress() as p, p.step("extract") as step:
                    prepare.get_extract(p, step, extract, refresh)
            self.assertEqual(extract.read_bytes(), BODY)
            self.assertEqual(seen, [None])
            self.assertEqual(step.detail, "0 MB")


class Main(Scratch):
    def run_main(self, *argv):
        extract = self.root / "q.pbf"
        extract.write_bytes(b"x" * 3_000_000)
        out = io.StringIO()
        patches = [
            mock.patch.object(sys, "argv", ["prepare.py", *argv]),
            mock.patch.object(sys, "stderr", out),
            mock.patch.object(metro_tiles, "LOGS", self.root / "logs"),
            mock.patch.object(metro_tiles, "run"),
            mock.patch.object(basemap_tiles, "run"),
        ]
        with contextlib.ExitStack() as stack:
            *_, metro, basemap = [stack.enter_context(patch) for patch in patches]
            prepare.main()
        return extract, out.getvalue(), metro, basemap

    def test_all_the_steps_are_listed_up_front(self):
        extract, text, metro, basemap = self.run_main(str(self.root / "q.pbf"))
        self.assertIn("PASS extract q.pbf 3 MB, kept", text)
        self.assertIn("Steps: 1 passed, 9 total", text)
        self.assertEqual(len(metro_tiles.STEPS) + len(basemap_tiles.STEPS), 8)
        self.assertEqual(metro.call_args.args[1:], (extract.resolve(), metro_tiles.WORK, metro_tiles.MAX_EXTRACTS))
        self.assertEqual(basemap.call_args.args[1], extract.resolve())

    def test_batch_goes_to_the_metro_tiles(self):
        _, _, metro, _ = self.run_main(str(self.root / "q.pbf"), "--batch", "2")
        self.assertEqual(metro.call_args.args[3], 2)

    def test_a_bad_batch_is_refused_before_anything_runs(self):
        for bad in ("0", "501", "many"):
            with self.subTest(bad), mock.patch.object(sys, "argv", ["prepare.py", "--batch", bad]):
                with mock.patch.object(sys, "stderr", io.StringIO()), self.assertRaises(SystemExit) as caught:
                    prepare.main()
                self.assertEqual(caught.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
