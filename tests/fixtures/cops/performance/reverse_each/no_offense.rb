[1, 2, 3].reverse_each { |x| puts x }
[1, 2, 3].each { |x| puts x }
[1, 2, 3].reverse
arr.reverse.map { |x| x.to_s }
arr.reverse_each(&:process)
arr.reverse.each.with_index { |e, i| puts i }
ret = arr.reverse.each { |x| x }
return arr.reverse.each { |e| e }
@ret = [1, 2, 3].reverse.each { |e| puts e }
[1, 2, 3].reverse.each { |e| puts e }.last
@cache ||= items.reverse.each { |x| process(x) }
@cache &&= items.reverse.each { |x| process(x) }
@@cache ||= items.reverse.each { |x| process(x) }
$cache ||= items.reverse.each { |x| process(x) }
CACHE ||= items.reverse.each { |x| process(x) }
Mod::CACHE ||= items.reverse.each { |x| process(x) }
