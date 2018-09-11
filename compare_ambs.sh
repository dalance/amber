#!/bin/sh

dev="./target/release/ambs --no-parent-ignore"
grep="grep --binary-files=without-match --color=auto -r"
ambs="ambs --no-parent-ignore"
rg="rg --no-heading --no-line-number"

hyperfine --warmup 3 "$dev  EXPORT_SYMBOL_GPL ./data/linux" \
                     "$ambs EXPORT_SYMBOL_GPL ./data/linux" \
                     "$rg   EXPORT_SYMBOL_GPL ./data/linux" \
                     "$grep EXPORT_SYMBOL_GPL ./data/linux"
hyperfine --warmup 3 "$dev  irq_bypass_register_producer ./data/linux" \
                     "$ambs irq_bypass_register_producer ./data/linux" \
                     "$rg   irq_bypass_register_producer ./data/linux" \
                     "$grep irq_bypass_register_producer ./data/linux"
hyperfine --warmup 3 "$dev  検索結果 ./data/jawiki-latest-pages-articles.xml" \
                     "$ambs 検索結果 ./data/jawiki-latest-pages-articles.xml" \
                     "$rg   検索結果 ./data/jawiki-latest-pages-articles.xml" \
                     "$grep 検索結果 ./data/jawiki-latest-pages-articles.xml"
hyperfine --warmup 3 "$dev  \"Quick Search\" ./data/jawiki-latest-pages-articles.xml" \
                     "$ambs \"Quick Search\" ./data/jawiki-latest-pages-articles.xml" \
                     "$rg   \"Quick Search\" ./data/jawiki-latest-pages-articles.xml" \
                     "$grep \"Quick Search\" ./data/jawiki-latest-pages-articles.xml"
