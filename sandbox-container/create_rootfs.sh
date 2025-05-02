#!/bin/bash
docker export $(docker create $1) --output="$2"