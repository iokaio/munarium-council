#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Send one bounded approval/transition request using a separately enrolled mTLS identity."""
import argparse
import json
from pathlib import Path
import ssl
import sys
import urllib.error
import urllib.request


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("redirect refused")


def send(endpoint, ca, certificate, key, request):
    if not endpoint.startswith("https://") or len(request) > 65536:
        raise ValueError("invalid endpoint or oversized request")
    json.loads(request)
    context = ssl.create_default_context(cafile=str(ca))
    context.load_cert_chain(str(certificate), str(key))
    opener = urllib.request.build_opener(NoRedirect(), urllib.request.HTTPSHandler(context=context))
    with opener.open(urllib.request.Request(endpoint, data=request,
                     headers={"Content-Type": "application/json"}), timeout=5) as response:
        raw = response.read(1048577)
        if len(raw) > 1048576:
            raise ValueError("response limit")
        return json.loads(raw)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", required=True)
    for name in ("ca", "certificate", "key", "request"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    try:
        result = send(args.endpoint, args.ca, args.certificate, args.key, args.request.read_bytes())
    except (OSError, ValueError, urllib.error.URLError):
        print("Council request refused or unavailable", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
