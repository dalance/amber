#!/bin/zsh

cnt=9
out="result.csv"

#prgs=(grep ack ag pt hw sift ambs)
prgs=(ambs)

opt_grep="--binary-files=without-match --color=auto -r"
opt_ack="--nogroup"
opt_ag="--nogroup"
opt_pt="--nogroup"
opt_hw="--no-group"
opt_sift=""
opt_ambs="--no-parent-ignore"

echo -n "" > $out

for p in $prgs; do
    eval `echo "$p --version >> $out"`
done

dir_info() {
    echo $1 >> $out
    du -sh $1 >> $out
    find $1 -type f | wc -l >> $out
}

time_avg() {
    eval `echo "( TIME=%e /usr/bin/time grep -r $2 $3 | wc -l ) >> $out"`;
    for p in $prgs; do
        echo -n "" > tmp
        opt=`eval echo '$opt_'$p`;
        eval `echo "( TIME=%e /usr/bin/time $p $opt $2 $3 )"`;
        for i in {0..$cnt}; do
            eval `echo "( TIME=%e /usr/bin/time $p $opt $2 $3 ) 2>> tmp"`;
        done;
        echo -n $1,$p, >> $out
        awk '{sum+=$1}END{print sum/NR}' tmp >> $out
    done
}

dir_info "./data/linux"
dir_info "./data/linux_build"
dir_info "./data/jawiki-latest-pages-articles.xml"
dir_info "./data/llvm"
dir_info "./data/jawiki-latest-abstract1.xml"

time_avg "many files / many hits"             "EXPORT_SYMBOL_GPL"            "./data/linux"
time_avg "many files / few hits"              "irq_bypass_register_producer" "./data/linux"
time_avg "many files / many hits with binary" "EXPORT_SYMBOL_GPL"            "./data/linux_build"
time_avg "many files / few hits with binary"  "irq_bypass_register_producer" "./data/linux_build"
time_avg "a large file / many hits"           "検索結果"                     "./data/jawiki-latest-pages-articles.xml"
time_avg "a large file / few hits"            "\"Quick Search\""             "./data/jawiki-latest-pages-articles.xml"
#time_avg "many files / many hits"             "i686"                         "./data/llvm"
#time_avg "many files / few hits"              "arm_neon_sha1"                "./data/llvm"
#time_avg "a large file / many hits"           "検索結果"                     "./data/jawiki-latest-abstract1.xml"
#time_avg "a large file / few hits"            "Rust"                         "./data/jawiki-latest-abstract1.xml"
