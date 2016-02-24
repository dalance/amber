#!/bin/zsh

cnt=10
out="result.txt"

#prgs=(grep ack ag pt hw sift ambs)
prgs=(grep ag pt hw sift ambs_20160208_dcef98d ambs_20160212_47fada7 ambs_20160215_988ba02 ambs_20160217_365746a ambs_20160218_8979031 ambs_20160219_9fec428)

time="TIME=%U,%S,%e,%P,%M,%F,%R,%w,%I,%O /usr/bin/time"

opt_grep="--binary-files=without-match --color=auto -r"
opt_ack="--nogroup"
opt_ag="--nogroup"
opt_pt="--nogroup"
opt_hw="--no-group"
opt_sift=""
opt_ambs=""

echo "" > $out

for p in $prgs; do
    eval `echo "$p --version >> $out"`
done

echo "bench,prg,user time[s],sys time[s],total time[s],cpu usage[%],mem usage[kB],major page fault,minor page fault,wait,in,out" >> $out

name="many files and many matches"
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        echo -n $name,$p, >> $out
        eval `echo "( $time $p $opt EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`;
    done;
done

name="many files and few matches"
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        echo -n $name,$p, >> $out
        eval `echo "( $time $p $opt irq_bypass_register_producer ./data/linux ) 2>> $out"`;
    done;
done

name="many files and many matches with binary"
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        echo -n $name,$p, >> $out
        eval `echo "( $time $p $opt EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`;
    done;
done

name="many files and few matches with binary"
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        echo -n $name,$p, >> $out
        eval `echo "( $time $p $opt irq_bypass_register_producer ./data/linux_build ) 2>> $out"`;
    done;
done

name="large file and many matches"
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        echo -n $name,$p, >> $out
        eval `echo "( $time $p $opt 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`;
    done;
done

name="large file and few matches"
for p in $prgs; do
    for i in {0..$cnt}; do
        opt=`eval echo '$opt_'$p`;
        echo -n $name,$p, >> $out
        eval `echo "( $time $p $opt \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`;
    done;
done

