#!/bin/bash
set -e
python3.12 -m venv env
source env/bin/activate
which python
pip install --upgrade pip
pip install -r requirements.txt