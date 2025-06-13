#!/bin/bash

ENGINE=./target/release/chess
PUZZLES=puzzles.txt
DEPTH=9
THRESHOLD=60000 # Tiempo límite en ms

export ENGINE DEPTH THRESHOLD

check_puzzle() {
    line="$1"
    id=$(echo "$line" | awk -F'[:,]' '{print $2}')
    fen=$(echo "$line" | awk -F'[:,]' '{print $4}' | sed 's/^ *//')
    move1=$(echo "$line" | awk -F'[:,]' '{print $6}' | awk '{print $1}')

    result=$(
        {
            echo "uci"
            echo "isready"
            echo "position fen $fen moves $move1"
            echo "go depth $DEPTH"
        } | $ENGINE
    )

    time_ms=$(echo "$result" | grep "^info depth" | tail -1 | awk '{for(i=1;i<=NF;i++) if($i=="time") print $(i+1)}')
    [ -z "$time_ms" ] && time_ms=0

    if ((time_ms > THRESHOLD)); then
        echo "$id"
    fi
}

export -f check_puzzle

cat "$PUZZLES" | parallel --jobs 16 --line-buffer check_puzzle
