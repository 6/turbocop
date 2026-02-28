[1, 2, 3].filter_map { |x| x > 1 ? x * 2 : nil }
[1, 2, 3].select { |x| x > 1 }
[1, 2, 3].map { |x| x * 2 }
arr.select { |x| x > 1 }.each { |x| puts x }
arr.select { |x| x > 1 }.count
ary.do_something.select(&:present?).stranger.map(&:to_i).max
ary.select { |o| o.present? }.stranger.map { |o| o.to_i }
ary.do_something.select(key: value).map(&:to_i)
# RuboCop skips numblock/it patterns (Parser gem's block_type? returns false for numblock)
arr.select { _1.valid? }.map { _1.name }
arr.select { _1 > 5 }.map { _1.to_s }.uniq
arr.filter { _1.present? }.map { _1.id }
# Ruby 3.4 `it` parameter also creates numblock in Parser gem
items.select { it.visible }.map { it.display_name }
items.filter { it.present? }.map { it.name }
