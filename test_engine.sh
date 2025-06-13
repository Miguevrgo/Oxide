#!/bin/bash

ENGINE=./target/release/chess
PUZZLES=puzzles.txt
DEPTH=7

echo "ID,FEN,Expected,EngineMove,Correct,Depth,Time(ms),Nodes,NPS" >resultados.csv

while IFS= read -r line; do
    id=$(echo "$line" | awk -F'[:,]' '{print $2}')
    fen=$(echo "$line" | awk -F'[:,]' '{print $4}' | sed 's/^ *//')
    move1=$(echo "$line" | awk -F'[:,]' '{print $6}' | awk '{print $1}')
    move2=$(echo "$line" | awk -F'[:,]' '{print $6}' | awk '{print $2}')

    result=$(
        {
            echo "uci"
            echo "isready"
            echo "position fen $fen moves $move1"
            echo "go depth $DEPTH"
        } | $ENGINE
    )

    bestmove=$(echo "$result" | grep bestmove | awk '{print $2}')
    last_info=$(echo "$result" | grep "^info depth" | tail -1)

    d=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="depth") d=$(i+1)} END{print d}')
    t=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="time") t=$(i+1)} END{print t}')
    n=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="nodes") n=$(i+1)} END{print n}')
    nps=$(echo "$last_info" | awk '{for(i=1;i<=NF;i++) if($i=="nps") nps=$(i+1)} END{print nps}')

    correct="NO"
    [ "$bestmove" == "$move2" ] && correct="YES"

    echo "$id,\"$fen\",$move2,$bestmove,$correct,$d,$t,$n,$nps" >>resultados.csv
done <"$PUZZLES"

echo "✅ Resultados guardados en resultados.csv"
