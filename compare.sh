#!/bin/zsh

cnt=9
out="result.txt"

opt_grep="--binary-files=without-match --color=auto -r"
opt_ag="--nogroup"
opt_pt="--nogroup"
opt_ambs0=""
opt_ambs1="--sse"
opt_ambs2="--tbm"
opt_ambs3="--tbm --sse"

echo "" > $out

grep --version >> $out
ag   --version >> $out
pt   --version >> $out
ambs --version >> $out

echo "\nmany files and many matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep  EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag    EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt    EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs0 EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs1 EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs2 EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs3 EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done

echo "\nmany files and few matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep  irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag    irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt    irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs0 irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs1 irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs2 irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs3 irq_bypass_register_producer ./data/linux ) 2>> $out"`; done

echo "\nmany files and many matches with binary" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep  EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag    EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt    EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs0 EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs1 EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs2 EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs3 EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done

echo "\nmany files and few matches with binary" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep  irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag    irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt    irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs0 irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs1 irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs2 irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs3 irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done

echo "\nlarge file and many matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep  検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag    検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt    検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs0 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs1 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs2 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs3 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done

echo "\nlarge file and few matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep  \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag    \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt    \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs0 \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs1 \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs2 \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs3 \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
