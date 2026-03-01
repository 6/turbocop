[1, 2, 3].filter_map { |x| x if x > 1 }
[1, 2, 3].compact
[1, 2, 3].map { |x| x * 2 }
arr.map.compact
arr.collect { |x| x }.first
# Numbered parameters (_1) — RuboCop skips numblock
items.map { _1.to_s }.compact
items.map { _1["price"] }.compact
items.collect { _1.alive? ? _1.id : nil }.compact
# It parameter (Ruby 3.4) — RuboCop skips itblock
items.map { it.to_s }.compact
# Block pass with method reference — RuboCop only matches &:symbol
items.map(&method(:transform)).compact
items.map(&attr).compact
items.collect(&method(:parse_uri)).compact
# map/collect with method arguments — not a simple Enumerable#map call
Parallel.map(items) { |item| transform(item) }.compact
Parallel.map(items, in_threads: 4) { |item| transform(item) }.compact
Helper.collect(records, batch_size: 100) { |r| r.name }.compact
