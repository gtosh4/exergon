#!/usr/bin/env bash
cd "$(dirname "$0")/ui_mock"
python3 -m http.server "${1:-8080}"
