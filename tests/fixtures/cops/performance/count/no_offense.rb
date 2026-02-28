[1, 2, 3].count { |x| x > 1 }
[1, 2, 3].count
arr.count
[1, 2, 3].map { |e| e + 1 }.size
[1, 2, 3].size
# select with string arg (Active Record)
Model.select('field AS field_one').count
# select with symbol arg (Active Record)
Model.select(:value).count
# interstitial method between select and count
array.select(&:value).uniq.count
# bang methods are not flagged
[1, 2, 3].select! { |e| e.odd? }.size
[1, 2, 3].reject! { |e| e.odd? }.count
# select...count with a block on count (allowed)
[1, 2, 3].select { |e| e.odd? }.count { |e| e > 2 }
# sole statement in a block body (RuboCop skips these)
items.map do |r|
  r.split(".").reject { |s| s == "*" }.count
end
items.map { |r| r.select { |s| s.valid? }.length }
run(:task) do |records|
  records.select { |r| r.active? }.size
end
change { items.select { |e| e.ready? }.count }
# empty block (no body) — RuboCop skips these
arr.select { }.count
arr.reject { }.size
# numbered parameters (_1) — RuboCop uses numblock which doesn't match block pattern
items.select { _1.positive? }.count
items.reject { _1 > 0 }.size
records.filter { _1.active? }.length
items.select { _1[1].positive? }.count
# it parameter (Ruby 3.4) — also numblock in parser-gem
records.filter { it.valid? }.length
items.select { it > 0 }.count
