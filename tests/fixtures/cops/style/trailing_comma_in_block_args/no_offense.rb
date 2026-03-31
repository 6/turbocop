foo { |a, b| a + b }
bar { |x| x }
baz { |item| item.to_s }
qux { puts 'hello' }
items.each { |i| puts i }
x = [1, 2, 3]

# Single parameter with trailing comma (destructuring) — NOT an offense
items.each { |item,| process(item) }
hash.each_key { |key,| keys << key }
[1, 2].map { |x, | x * 2 }
test { |a,| a }
test do |a,|
  a
end
define_method(:m) { |a,| a }

# Chained blocks remain fine when the receiver block does not have
# a trailing comma in its single parameter list.
items.sort_by { |name| name }.map do |name, value|
  value.to_s
end
data.select { |k| allowed?(k) }.each do |k, v|
  process(k, v)
end
groups.sort_by do |day| day end.reverse_each do |day, entries|
  display(day, entries)
end

# When an earlier block in the receiver chain has piped params (even without
# trailing comma), RuboCop's token-based approach picks those pipes first,
# so a later single-param trailing-comma block is masked — not an offense.
symbols
  .to_h { |sym| [sym, nil] }.merge(symbols_with_args)
  .map { |sym, args| [sym, args] }
  .each { |name,|
    puts name
  }
  .map { |name, args| [name, args] }
