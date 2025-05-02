#!/bin/bash
set -e -x
cd "$(dirname "$0")/.."
python -m pip install -r requirements.txt
python -m unittest discover -v
flake8 . --max-line-length=128 --exclude env/
mypy . --exclude ./env
