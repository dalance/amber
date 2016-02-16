#!/bin/zsh

cnt=3
out="result.txt"

#prgs=(grep ack ag pt hw sift ambs)
prgs=(grep ag pt hw sift ambs)

opt_grep="--binary-files=without-match --color=auto -r"
opt_ack="--nogroup"
opt_ag="--nogroup"
opt_pt="--nogroup"
opt_hw="--no-group"
opt_sift=""
#opt_ambs="--no-skip-gitignore"
opt_ambs=""

echo "" > $out

for p in $prgs; do
    eval `echo "$p --version >> $out"`
done

echo "\nmany files and many matches" >> $out
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        eval `echo "( time $p $opt EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`;
    done;
done

echo "\nmany files and few matches" >> $out
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        eval `echo "( time $p $opt irq_bypass_register_producer ./data/linux ) 2>> $out"`;
    done;
done

echo "\nmany files and many matches with binary" >> $out
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        eval `echo "( time $p $opt EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`;
    done;
done

echo "\nmany files and few matches with binary" >> $out
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        eval `echo "( time $p $opt irq_bypass_register_producer ./data/linux_build ) 2>> $out"`;
    done;
done

echo "\nlarge file and many matches" >> $out
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        eval `echo "( time $p $opt 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`;
    done;
done

echo "\nlarge file and few matches" >> $out
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        eval `echo "( time $p $opt \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`;
    done;
done

