#!/bin/sh

info=`github-release info --user dalance --repo amber | grep $CIRCLE_TAG`

if [ info -eq "" ]; then
    github-release release \
        --user dalance     \
        --repo amber       \
        --tag  $CIRCLE_TAG \
        --name $CURCLE_TAG
fi

for i in $(ls -1 *.zip)
do
    github-release upload  \
        --user dalance     \
        --repo amber       \
        --tag  $CIRCLE_TAG \
        --name "$i"        \
        --file  $i
done
