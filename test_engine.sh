#!/bin/bash

ENGINE=./target/release/chess
PUZZLES=puzzles.txt
DEPTH=9
LIMIT=${1:-ALL} # Default is ALL

echo "ID,FEN,Expected,EngineMove,Correct,Depth,Time(ms),Nodes,NPS" >results.csv

total_time_ms=0
correct=0
total=0

start_all=$(date +%s%3N)

if [[ "$LIMIT" == "ALL" ]]; then
    mapfile -t lines <"$PUZZLES"
else
    mapfile -t lines < <(head -n "$LIMIT" "$PUZZLES")
fi

for line in "${lines[@]}"; do
    id=$(echo "$line" | awk -F'[:,]' '{print $2}')
    fen=$(echo "$line" | awk -F'[:,]' '{print $4}' | sed 's/^ *//')
    move1=$(echo "$line" | awk -F'[:,]' '{print $6}' | awk '{print $1}')
    move2=$(echo "$line" | awk -F'[:,]' '{print $6}' | awk '{print $2}')

    result=$(
        {
            echo "uci"
            echo "isready"
            echo "position fen $fen moves $move1"
            echo "go"
        } | $ENGINE
    )

    bestmove=$(echo "$result" | grep bestmove | awk '{print $2}')
    last_info=$(echo "$result" | grep "^info depth" | tail -1)

    d=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="depth") d=$(i+1)} END{print d}')
    t=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="time") t=$(i+1)} END{print t}')
    n=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="nodes") n=$(i+1)} END{print n}')
    nps=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="nps") nps=$(i+1)} END{print nps}')

    [ -z "$t" ] && t=0

    correct_flag="NO"
    if [ "$bestmove" == "$move2" ]; then
        correct_flag="YES"
        ((correct++))
    fi

    ((total++))
    ((total_time_ms += t))

    echo "$id,\"$fen\",$move2,$bestmove,$correct_flag,$d,$t,$n,$nps" >>results.csv
done

end_all=$(date +%s%3N)
elapsed_all=$((end_all - start_all))
avg_time=$((total_time_ms / total))

echo
echo "󰞌  Tests completed in $elapsed_all ms"
echo "⏱️  Average time per puzzle: $avg_time ms"
echo "✅ Correct puzzles: [$correct/$total]"
