#!/usr/bin/env bash
set -euo pipefail

shopt -s nullglob
files=(/ttl/*.ttl.gz)
shopt -u nullglob

total=${#files[@]}

for i in "${!files[@]}"; do
  file="${files[$i]}"
  name=$(basename "$file" .ttl.gz)
  echo -e "\033[1m[$((i+1))/${total}] Creating index for '$name'\033[0m"
  QLEVER_NAME="$name" QLEVER_INPUT_FILES="../${file}" qlever index --overwrite-existing
done
