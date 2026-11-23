"""The software factory's configuration and GitHub App token (.claude/factory/factory.py): `just test-scripts`.

The factory reads its Linear settings from .claude/factory/config.json and the App, run, evidence and deploy settings
from .github/issue-flow.json, and acts on GitHub as the App so that a person can approve its pull requests.
"""

from __future__ import annotations

import base64
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / ".claude" / "factory"))

import factory  # noqa: E402


def b64url_decode(part: str) -> bytes:
    return base64.urlsafe_b64decode(part + "=" * (-len(part) % 4))


class Config(unittest.TestCase):
    def test_the_repositorys_configuration_is_valid(self):
        config = factory.load_config(ROOT)
        self.assertEqual(config["linear"]["team"], "Cityloom")
        self.assertEqual(config["github"]["repo"], "alandovskis/cityloom")
        self.assertTrue(config["app"]["privateKeyPath"].startswith("/"))
        self.assertTrue(config["run"].strip() and config["evidence"].strip())
        # Todo empty, the queue takes from these states, in this order; the intake is never one of them.
        self.assertEqual(config["linear"]["fallback"], ["Triage", "Backlog"])

    def test_every_label_the_factory_applies_is_one_github_knows(self):
        # Linear's S- labels mirror .github/labels.yml: the factory's words must be the same in both trackers.
        labels = (ROOT / ".github" / "labels.yml").read_text(encoding="utf-8")
        for name in factory.load_config(ROOT)["linear"]["labels"].values():
            with self.subTest(name):
                self.assertIn(f'- name: "{name}"', labels)

    def test_each_problem_is_named(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".claude" / "factory").mkdir(parents=True)
            (root / ".github").mkdir()
            good = json.loads((ROOT / ".claude" / "factory" / "config.json").read_text(encoding="utf-8"))
            del good["linear"]["team"]
            good["caps"]["inFlight"] = 0
            good["linear"]["fallback"] = ["Triage", " "]
            (root / ".claude" / "factory" / "config.json").write_text(json.dumps(good), encoding="utf-8")
            (root / ".github" / "issue-flow.json").write_text(json.dumps({"app": {"id": 1}}), encoding="utf-8")
            with self.assertRaises(factory.ConfigError) as caught:
                factory.load_config(root)
        problems = caught.exception.problems
        self.assertIn("linear.team is missing", problems)
        self.assertIn("caps.inFlight must be a whole number above 0", problems)
        self.assertIn("linear.fallback must be a list of texts that are not blank", problems)
        self.assertIn("app.installationId is missing", problems)
        self.assertIn("app.privateKeyPath is missing", problems)
        self.assertIn("run is missing", problems)


@unittest.skipUnless(shutil.which("openssl"), "needs openssl")
class Token(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.key = Path(self.tmp.name) / "app.pem"
        self.public = Path(self.tmp.name) / "app.pub"
        subprocess.run(["openssl", "genrsa", "-out", str(self.key), "2048"], check=True, capture_output=True)
        subprocess.run(["openssl", "rsa", "-in", str(self.key), "-pubout", "-out", str(self.public)], check=True, capture_output=True)

    def tearDown(self):
        self.tmp.cleanup()

    def test_the_app_jwt_is_rs256_backdated_and_short_lived(self):
        jwt = factory.app_jwt(1234, str(self.key), now=1_000_000)
        header, claims, signature = jwt.split(".")
        self.assertEqual(json.loads(b64url_decode(header)), {"alg": "RS256", "typ": "JWT"})
        self.assertEqual(json.loads(b64url_decode(claims)), {"iat": 999_940, "exp": 1_000_540, "iss": 1234})
        signed = Path(self.tmp.name) / "signed"
        sig = Path(self.tmp.name) / "sig"
        signed.write_bytes(f"{header}.{claims}".encode())
        sig.write_bytes(b64url_decode(signature))
        verified = subprocess.run(
            ["openssl", "dgst", "-sha256", "-verify", str(self.public), "-signature", str(sig), str(signed)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(verified.returncode, 0, verified.stdout + verified.stderr)

    def test_the_installation_token_is_asked_for_with_the_jwt(self):
        asked = {}

        def post(url, headers):
            asked.update(url=url, headers=headers)
            return 201, json.dumps({"token": "ghs_abc", "expires_at": "2026-11-22T08:00:00Z"})

        app = {"id": 1234, "installationId": 99, "privateKeyPath": str(self.key)}
        self.assertEqual(factory.installation_token(app, post=post, now=1_000_000), "ghs_abc")
        self.assertEqual(asked["url"], "https://api.github.com/app/installations/99/access_tokens")
        self.assertTrue(asked["headers"]["Authorization"].startswith("Bearer ey"))

    def test_a_refusal_says_what_github_answered(self):
        app = {"id": 1, "installationId": 2, "privateKeyPath": str(self.key)}
        with self.assertRaises(factory.ConfigError) as caught:
            factory.installation_token(app, post=lambda url, headers: (401, '{"message":"Bad credentials"}'), now=1)
        self.assertIn("HTTP 401", caught.exception.problems[0])

    def test_an_unreadable_key_is_named(self):
        app = {"id": 1, "installationId": 2, "privateKeyPath": "/nowhere/key.pem"}
        with self.assertRaises(factory.ConfigError) as caught:
            factory.installation_token(app, post=lambda url, headers: (201, "{}"), now=1)
        self.assertIn("/nowhere/key.pem", caught.exception.problems[0])


if __name__ == "__main__":
    unittest.main()
