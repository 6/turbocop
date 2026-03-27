# No compact chained
ary.map { |x| x if x > 1 }
# Map with non-conditional body
ary.map { |x| x.to_s }.compact
ary.map(&:to_s).compact
# Just compact
ary.compact
# Return value is not the block parameter — not replaceable with select/reject
ary.map { |c| Regexp.last_match(1) if c =~ /pattern/ }.compact
ary.map { |x| x.upcase if x.valid? }.compact
ary.map { |item| transform(item) if item.present? }.compact
# compact with arguments
ary.map do |item|
  if item.bar?
    item
  else
    next
  end
end.compact(arg)
# Return value is not same as block argument
ary.map do |item|
  if item.bar?
    1
  else
    2
  end
end.compact
# elsif chain — vendor skips these
ary.map do |item|
  if item.bar?
    item
  elsif item.baz?
    item
  else
    next
  end
end.compact
# Map with other method chained (not compact)
ary.map do |item|
  if item.bar?
    item
  else
    next
  end
end.do_something
# Multi-parameter blocks — returning one param is not equivalent to select/reject
# RuboCop's node matcher requires exactly one block argument: (args $(arg _))
options.map do |key, value|
  key if value.present?
end.compact
hash.map { |k, v| k if v > 0 }.compact
items.filter_map do |name, spec|
  name if spec.metadata['default']
end
# Multi-parameter with unless
pairs.map { |a, b| a unless b.nil? }.compact
# filter_map with arguments — not Enumerable#filter_map, e.g. Parallel.filter_map(args)
# RuboCop's node pattern (call _ {:map :filter_map}) requires no arguments
Parallel.filter_map(['a', 'b', 'c']) do |x|
  x if x != 'b'
end
# Ternary with nil literal — RuboCop does NOT match nil literal in else
# (its pattern uses nil? which checks absent, not NilNode)
ary.map { |x| x.valid? ? x : nil }.compact
# Same pattern in block form
ary.map do |key|
  key.verify ? key : nil
end.compact
