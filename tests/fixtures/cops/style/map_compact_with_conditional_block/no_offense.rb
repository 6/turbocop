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
