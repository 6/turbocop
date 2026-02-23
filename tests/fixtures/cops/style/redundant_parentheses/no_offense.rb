x = (1 + 2)
z = (foo ? bar : baz)
w = (a || b) ? 1 : 2
result = method_call(arg)
arr = [1, 2, 3]
# Chained parens
x = (a && b).to_s
# Splat
foo(*args)
# do..end block in argument to unparenthesized method call — parens are required
# to prevent Ruby from binding the block to the outer method
scope :advisory_lock, (lambda do |column:|
  column
end)
scope :display_all, (lambda do |after_id: nil|
  where(id: after_id)
end)
has_many :items, (proc do
  order(:position)
end)
# break/return/next with adjacent parens — keyword directly touching open paren
break(value) unless value
return(result) if done
next(item) if skip
# do..end blocks in hash values — parens prevent block binding to outer method
foo(default: (lambda do |routes|
  routes
end))
bar(key: (proc do
  something
end))
# Assignment in boolean context — parens disambiguate = from ==
(results[:dump_called] = true) && "dump_something"
(results[:load_called] = true) && "load_something"
x = (y = 1) && z
(a = foo) || bar
# Comparison inside another expression — not top-level, not flagged
x = (a == b) ? 1 : 2
result = (a > b) && c
# Comparison inside method body (return value) — has parent, not flagged
def edited?
  (last_edited_at - created_at > 1.minute)
end
# Comparison as hash value — has parent, not flagged
config = { enable_starttls: (ENV["VAR"] == "true") }
# Range literals — parens around ranges are almost never redundant
arr = [(1..5)]
ranges + [(line..line)]
(minimum..maximum).cover?(count)
foo((1..10))
x = (0..10)
process((start..length), path, file)
