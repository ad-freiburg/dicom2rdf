#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

files=(/data/*.meta-data.json)
total=${#files[@]}

for i in "${!files[@]}"; do
  file="${files[$i]}"
  name=$(basename "$file" .meta-data.json)
  QLEVER_NAME="$name" qlever start
  max_depth=$(
    curl -s -G "http://localhost:7055" \
      --data-urlencode "query=SELECT ?d WHERE { ?s <meta:maxDepth> ?d }" |
    jq -r '.results.bindings[0].d.value'
  )
  echo -e "\033[1m[$((i+1))/${total}] Constructing semantic triples from '$name' (max depth: $max_depth)\033[0m"
  /app/construct \
    --config /app/config.toml \
    --prefix "$name" --output /ttl \
    --max-depth "$max_depth"
  QLEVER_NAME="$name" qlever stop
done

for f in /ttl-static/*.ttl; do
  gzip -c "$f" >/ttl/$(basename "$f").gz
done
