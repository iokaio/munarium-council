#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Real Council TLS/process tests with independently controlled synthetic dependency servers.

These component tests do not substitute for the full Gate/Server/Harness composition.
All keys and databases are disposable; no credential or raw request is printed.
"""
import copy
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import socket
import ssl
import subprocess
import tempfile
import threading
import time
import unittest
import urllib.error

from council_client import send

ROOT = Path(__file__).resolve().parents[1]


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(domain, value):
    return "sha256:" + hashlib.sha256(("munarium:stage2:" + domain + ":v1\0" + encoded(value)).encode()).hexdigest()


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


class Service(unittest.TestCase):
    def test_authenticated_approval_restart_and_refusal(self):
        openssl = shutil.which("openssl")
        if not openssl and os.name == "nt":
            openssl = "C:/Program Files/Git/usr/bin/openssl.exe"
        self.assertTrue(openssl and Path(openssl).is_file(), "OpenSSL required")
        binary = Path(os.environ.get("COUNCIL_TEST_BINARY", str(ROOT / "target/debug/munarium-council")))
        if os.name == "nt":
            binary = binary.with_suffix(".exe")
        self.assertTrue(binary.is_file(), "build Council before service tests")
        with tempfile.TemporaryDirectory(prefix="munarium-council-component-") as folder:
            root = Path(folder)

            def run(*args):
                subprocess.run([openssl, *map(str, args)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

            run("req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", root / "ca.key", "-out", root / "ca.pem", "-days", "1", "-subj", "/CN=council-component-ca", "-addext", "basicConstraints=critical,CA:TRUE", "-addext", "keyUsage=critical,keyCertSign,cRLSign")
            extension = root / "extension.cnf"
            extension.write_text("basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nsubjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth,clientAuth\n")
            fingerprints = {}
            for name in ("council", "dependency", "human", "agent", "unknown"):
                run("req", "-newkey", "rsa:2048", "-nodes", "-keyout", root / (name + ".key"), "-out", root / (name + ".csr"), "-subj", "/CN=" + name)
                run("x509", "-req", "-in", root / (name + ".csr"), "-CA", root / "ca.pem", "-CAkey", root / "ca.key", "-CAcreateserial", "-days", "1", "-extfile", extension, "-out", root / (name + ".pem"))
                fingerprints[name] = hashlib.sha256(ssl.PEM_cert_to_DER_cert((root / (name + ".pem")).read_text())).hexdigest()
            vectors = json.loads((ROOT / "contracts/stage2-v1/vectors.json").read_text())
            request = copy.deepcopy(vectors["records"]["request"])
            now = int(time.time())
            request["context"]["valid_from"] = now - 10
            request["context"]["expires_at"] = now + 290
            request["context"]["evidence"][0].update(observed_at=now - 10, expires_at=now + 290)
            request["context_digest"] = digest("context", request["context"])
            decision = copy.deepcopy(vectors["records"]["decision"])
            decision.update(request_digest=digest("action-request", request), context_digest=request["context_digest"])
            scope = vectors["trusted"]["scope"]
            policy = dict(scope=scope, humans={"human": dict(principal=vectors["trusted"]["approver"], eligibility_revision=1)}, readers=["agent"], stream="council-events", generation=1)

            class Dependency(BaseHTTPRequestHandler):
                def log_message(self, *args):
                    pass

                def answer(self, payload):
                    if hashlib.sha256(self.connection.getpeercert(binary_form=True)).hexdigest() != fingerprints["council"]:
                        self.send_error(403)
                        return
                    raw = encoded(payload).encode()
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(raw)))
                    self.end_headers()
                    self.wfile.write(raw)

                def do_GET(self):
                    self.answer(dict(config=dict(deployment=scope["deployment"], tenant=scope["tenant"]), head=1, artifact=dict(bindings={"stage2:council": policy})))

                def do_POST(self):
                    count = int(self.headers.get("Content-Length", "0"))
                    if count > 65536:
                        self.send_error(413)
                        return
                    self.rfile.read(count)
                    self.answer(dict(request=request, decision=decision, requester_chain=[request["intent"]["actor"]], current_context=request["context"]))

            dependency = ThreadingHTTPServer(("127.0.0.1", 0), Dependency)
            tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
            tls.load_cert_chain(root / "dependency.pem", root / "dependency.key")
            tls.load_verify_locations(root / "ca.pem")
            tls.verify_mode = ssl.CERT_REQUIRED
            dependency.socket = tls.wrap_socket(dependency.socket, server_side=True)
            thread = threading.Thread(target=dependency.serve_forever, daemon=True)
            thread.start()
            port = free_port()
            upstream = "https://localhost:" + str(dependency.server_port)
            config = dict(tls=dict(listen="127.0.0.1:" + str(port), certificate_file=str(root / "council.pem"), private_key_file=str(root / "council.key"), ca_file=str(root / "ca.pem"), peers={fingerprints[p]: dict(service=p, tenants=[scope["tenant"]]) for p in ("human", "agent")}), database=str(root / "council.sqlite"), server_endpoint=upstream, gate_endpoint=upstream, warden_endpoint=upstream, deployment=scope["deployment"], service="council", server_service="server", provider_id="fixture", provider_token_file=str(root / "unused-token"))
            (root / "config.json").write_text(encoded(config))
            process = None

            def start():
                child = subprocess.Popen([str(binary), str(root / "config.json")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                for _ in range(100):
                    if child.poll() is not None:
                        self.fail("Council exited before readiness")
                    try:
                        with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                            return child
                    except OSError:
                        time.sleep(0.02)
                child.kill()
                child.wait()
                self.fail("Council startup timeout")

            def call(peer, action, tenant=None):
                return send("https://localhost:" + str(port) + "/v1/approvals", root / "ca.pem", root / (peer + ".pem"), root / (peer + ".key"), encoded(dict(tenant=tenant or scope["tenant"], action=action)).encode())

            try:
                process = start()
                action = dict(operation="approve", id="approval-service", operation_id="publish-artifact", attempt_id="attempt-a")
                first = call("human", action)
                for peer, tenant in (("agent", None), ("human", "tenant-b"), ("unknown", None)):
                    with self.assertRaises((OSError, ValueError, urllib.error.URLError)):
                        call(peer, action, tenant)
                process.terminate()
                process.wait(timeout=10)
                process = start()
                self.assertEqual(call("human", action)["approval"], first["approval"])
                policy["humans"]["human"]["eligibility_revision"] = 2
                self.assertFalse(call("agent", dict(operation="lookup", id="approval-service"))["currently_usable"])
            finally:
                if process and process.poll() is None:
                    process.terminate()
                    process.wait(timeout=10)
                dependency.shutdown()
                dependency.server_close()
                thread.join(timeout=10)


if __name__ == "__main__":
    unittest.main()
