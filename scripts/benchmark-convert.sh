#!/bin/sh
set -eu

INPUT_ROOT=${INPUT_ROOT:-/input}
OUTPUT_ROOT=${OUTPUT_ROOT:-/output}
CONFIG_PATH=${CONFIG_PATH:-/app/config.toml}
CHUNK_SIZE=${CHUNK_SIZE:-50000000}
MAX_TRIPLES_PER_FILE=${MAX_TRIPLES_PER_FILE:-100000000}
MAX_THREADS=${MAX_THREADS:-}
RUNS_TOTAL=12
WARMUP_RUNS=2

REPORT_PATH=${REPORT_PATH:-$OUTPUT_ROOT/report.csv}
LOG_DIR=${LOG_DIR:-$OUTPUT_ROOT/logs}
RUNS_DIR=${RUNS_DIR:-$OUTPUT_ROOT/runs}

if [ ! -d "$INPUT_ROOT" ]; then
    echo "Input root not found: $INPUT_ROOT" >&2
    exit 1
fi

mkdir -p "$OUTPUT_ROOT" "$LOG_DIR" "$RUNS_DIR"

if [ -z "$MAX_THREADS" ]; then
    if command -v nproc >/dev/null 2>&1; then
        MAX_THREADS=$(nproc)
    elif [ -r /proc/cpuinfo ]; then
        MAX_THREADS=$(grep -c '^processor' /proc/cpuinfo || true)
    else
        MAX_THREADS=1
    fi
fi

if [ "$MAX_THREADS" -lt 1 ]; then
    MAX_THREADS=1
fi

printf '%s\n' \
    "scenario_id,input_dir,output_dir,num_threads,compression_level,avg_duration_ms" \
    >"$REPORT_PATH"

list_input_dirs() {
    for d in "$INPUT_ROOT"/*; do
        [ -d "$d" ] || continue
        base=$(basename "$d")
        printf '%s:%s\n' "$base" "$d"
    done | sort -n -t: -k1,1 | cut -d: -f2-
}

input_dirs=$(list_input_dirs)
if [ -z "$input_dirs" ]; then
    echo "No input directories found under $INPUT_ROOT" >&2
    exit 1
fi

echo "Benchmark running..."

first_dir=$(printf '%s\n' "$input_dirs" | sed -n '1p')
rest_dirs=$(printf '%s\n' "$input_dirs" | sed -n '2,$p')

scenario_idx=0
next_scenario_id() {
    scenario_idx=$((scenario_idx + 1))
    scenario_id=$(printf '%03d' "$scenario_idx")
}

run_convert() {
    input_dir=$1
    num_threads=$2
    compression=$3

    next_scenario_id
    input_label=$(basename "$input_dir")
    scenario_tag="scenario-${scenario_id}_input-${input_label}_threads-${num_threads}_comp-${compression}"
    scenario_output_dir="$RUNS_DIR/$scenario_tag"
    scenario_log_dir="$LOG_DIR/$scenario_tag"
    kept_sum=0
    kept_ok=0
    any_error=0

    mkdir -p "$scenario_output_dir" "$scenario_log_dir"

    iter=1
    while [ "$iter" -le "$RUNS_TOTAL" ]; do
        iter_tag=$(printf '%02d' "$iter")
        log_path="$scenario_log_dir/run-${iter_tag}.log"

        if duration_ms=$(/app/convert \
            --config "$CONFIG_PATH" \
            --input-dir "$input_dir" \
            --output-dir "$scenario_output_dir" \
            --chunk-size "$CHUNK_SIZE" \
            --max-triples-per-file "$MAX_TRIPLES_PER_FILE" \
            --compression-level "$compression" \
            --num-threads "$num_threads" \
            --benchmark \
            2>"$log_path"); then
            ok=1
        else
            ok=0
            any_error=1
        fi

        if [ "$iter" -gt "$WARMUP_RUNS" ]; then
            if [ "$ok" -eq 1 ]; then
                kept_ok=$((kept_ok + 1))
                kept_sum=$((kept_sum + duration_ms))
            else
                any_error=1
            fi
        fi

        iter=$((iter + 1))
    done

    if [ "$kept_ok" -eq $((RUNS_TOTAL - WARMUP_RUNS)) ] && [ "$any_error" -eq 0 ]; then
        avg_duration=$((kept_sum / kept_ok))
    else
        avg_duration=na
    fi

    printf '%s,%s,%s,%s,%s,%s\n' \
        "$scenario_id" "$input_dir" "$scenario_output_dir" "$num_threads" "$compression" \
        "$avg_duration" \
        >>"$REPORT_PATH"
}

threads=1
while [ "$threads" -le "$MAX_THREADS" ]; do
    run_convert "$first_dir" "$threads" 0
    threads=$((threads + 1))
done

compression=1
while [ "$compression" -le 9 ]; do
    run_convert "$first_dir" "$MAX_THREADS" "$compression"
    compression=$((compression + 1))
done

if [ -n "$rest_dirs" ]; then
    printf '%s\n' "$rest_dirs" | while IFS= read -r dir; do
        [ -n "$dir" ] || continue
        run_convert "$dir" "$MAX_THREADS" 0
    done
fi

echo "Benchmark done."
