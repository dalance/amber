#!/bin/zsh

cnt=3
out="result.txt"

opt_grep="--binary-files=without-match --color=auto -r"
opt_ack="--nogroup"
opt_ag="--nogroup"
opt_pt="--nogroup"
opt_hw="--no-group"
opt_sift=""
opt_ambs=""

echo "" > $out

grep --version >> $out
ack  --version >> $out
ag   --version >> $out
pt   --version >> $out
hw   --version >> $out
sift --version >> $out
ambs --version >> $out

echo "\nmany files and many matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
#for i in {0..$cnt}; do eval `echo "( time ack  $opt_ack  EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag   EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt   EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time hw   $opt_hw   EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time sift $opt_sift EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs EXPORT_SYMBOL_GPL ./data/linux ) 2>> $out"`; done

echo "\nmany files and few matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
#for i in {0..$cnt}; do eval `echo "( time ack  $opt_ack  irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag   irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt   irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time hw   $opt_hw   irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time sift $opt_sift irq_bypass_register_producer ./data/linux ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs irq_bypass_register_producer ./data/linux ) 2>> $out"`; done

echo "\nmany files and many matches with binary" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
#for i in {0..$cnt}; do eval `echo "( time ack  $opt_ack  EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag   EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt   EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time hw   $opt_hw   EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time sift $opt_sift EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs EXPORT_SYMBOL_GPL ./data/linux_build ) 2>> $out"`; done

echo "\nmany files and few matches with binary" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
#for i in {0..$cnt}; do eval `echo "( time ack  $opt_ack  irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag   irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt   irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time hw   $opt_hw   irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time sift $opt_sift irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs irq_bypass_register_producer ./data/linux_build ) 2>> $out"`; done

echo "\nlarge file and many matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
#for i in {0..$cnt}; do eval `echo "( time ack  $opt_ack  検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag   検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt   検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time hw   $opt_hw   検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time sift $opt_sift 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs 検索結果 ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done

echo "\nlarge file and few matches" >> $out
for i in {0..$cnt}; do eval `echo "( time grep $opt_grep \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
#for i in {0..$cnt}; do eval `echo "( time ack  $opt_ack  \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ag   $opt_ag   \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time pt   $opt_pt   \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time hw   $opt_hw   \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time sift $opt_sift \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
for i in {0..$cnt}; do eval `echo "( time ambs $opt_ambs \"Quick Search\" ./data/jawiki-latest-pages-articles.xml ) 2>> $out"`; done
