#!/bin/sh

dev="./target/release/ambr --no-interactive"
ambr="ambr --no-interactive"
fastmod="fastmod --accept-all"

hyperfine --warmup 3 "$dev     EXPORT_SYMBOL_GPL EXPORT_SYMBOL_GPL2 ./data/linux; $dev     EXPORT_SYMBOL_GPL2 EXPORT_SYMBOL_GPL ./data/linux" \
                     "$ambr    EXPORT_SYMBOL_GPL EXPORT_SYMBOL_GPL2 ./data/linux; $ambr    EXPORT_SYMBOL_GPL2 EXPORT_SYMBOL_GPL ./data/linux" \
                     "$fastmod EXPORT_SYMBOL_GPL EXPORT_SYMBOL_GPL2 ./data/linux; $fastmod EXPORT_SYMBOL_GPL2 EXPORT_SYMBOL_GPL ./data/linux" \
                     "find ./data/linux -type f | xargs sed -i 's/EXPORT_SYMBOL_GPL/EXPORT_SYMBOL_GPL2/g'; find ./data/linux -type f | xargs sed -i 's/EXPORT_SYMBOL_GPL2/EXPORT_SYMBOL_GPL/g'"
hyperfine --warmup 3 "$dev     irq_bypass_register_producer irq_bypass_register_producer2 ./data/linux; $dev     irq_bypass_register_producer2 irq_bypass_register_producer ./data/linux" \
                     "$ambr    irq_bypass_register_producer irq_bypass_register_producer2 ./data/linux; $ambr    irq_bypass_register_producer2 irq_bypass_register_producer ./data/linux" \
                     "$fastmod irq_bypass_register_producer irq_bypass_register_producer2 ./data/linux; $fastmod irq_bypass_register_producer2 irq_bypass_register_producer ./data/linux" \
                     "find ./data/linux -type f | xargs sed -i 's/irq_bypass_register_producer/irq_bypass_register_producer2/g'; find ./data/linux -type f | xargs sed -i 's/irq_bypass_register_producer2/irq_bypass_register_producer/g'"
hyperfine --warmup 3 "$dev     検索結果 検索結果2 ./data/jawiki-latest-pages-articles.xml; $dev     検索結果2 検索結果 ./data/jawiki-latest-pages-articles.xml" \
                     "$ambr    検索結果 検索結果2 ./data/jawiki-latest-pages-articles.xml; $ambr    検索結果2 検索結果 ./data/jawiki-latest-pages-articles.xml" \
                     "$fastmod 検索結果 検索結果2 ./data/jawiki-latest-pages-articles.xml; $fastmod 検索結果2 検索結果 ./data/jawiki-latest-pages-articles.xml" \
                     "find ./data/jawiki-latest-pages-articles.xml -type f | xargs sed -i 's/検索結果/検索結果2/g'; find ./data/jawiki-latest-pages-articles.xml -type f | xargs sed -i 's/検索結果2/検索結果/g'"
hyperfine --warmup 3 "$dev     \"Quick Search\" \"Quick Search2\" ./data/jawiki-latest-pages-articles.xml; $dev     \"Quick Search2\" \"Quick Search\" ./data/jawiki-latest-pages-articles.xml" \
                     "$ambr    \"Quick Search\" \"Quick Search2\" ./data/jawiki-latest-pages-articles.xml; $ambr    \"Quick Search2\" \"Quick Search\" ./data/jawiki-latest-pages-articles.xml" \
                     "$fastmod \"Quick Search\" \"Quick Search2\" ./data/jawiki-latest-pages-articles.xml; $fastmod \"Quick Search2\" \"Quick Search\" ./data/jawiki-latest-pages-articles.xml" \
                     "find ./data/jawiki-latest-pages-articles.xml -type f | xargs sed -i 's/\"Quick Search\"/\"Quick Search2\"/g'; find ./data/jawiki-latest-pages-articles.xml -type f | xargs sed -i 's/\"Quick Search2\"/\"Quick Search\"/g'"
