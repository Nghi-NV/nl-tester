#!/usr/bin/env python3
import sys
import json
import os

def main():
    device = sys.argv[1] if len(sys.argv) > 1 else "default_device"
    mode = os.getenv("TEST_MODE", "standard")

    output = {
        "status": "SUCCESS",
        "device": device,
        "mode": mode,
        "session_id": "sess_8839210",
        "data": {
            "token": "token_abc_123_xyz",
            "user_id": "user_7712"
        }
    }
    print(json.dumps(output))

if __name__ == "__main__":
    main()
