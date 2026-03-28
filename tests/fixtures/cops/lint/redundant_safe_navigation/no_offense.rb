CONST&.do_something
CONST_NAME&.do_something
nil&.to_i
foo&.bar
foo&.respond_to?(:to_a)
foo&.to_s&.zero?
foo&.to_i&.zero?
foo&.to_a&.zero?
foo&.to_h&.zero?
foo.bar
foo&.to_s || 'Default string'
foo&.to_i || 1
foo&.to_f || 1.0
foo&.to_a || [1]
foo&.to_h || { a: 1 }
foo&.to_i(16) || 0
bar&.to_s(:db) || ''

# AllowedMethods outside of conditions — no offense
foo&.respond_to?(:bar)
foo&.is_a?(String)
foo&.kind_of?(Hash)
if snags&.present?
end

# AllowedMethods inside body of if (not in predicate) — no offense
if condition
  foo&.respond_to?(:bar)
end

# Non-allowed method in condition — no offense
do_something if foo&.bar?

# respond_to? with nil-specific method as argument in condition — no offense
do_something if foo&.respond_to?(:to_a)
do_something if foo&.respond_to?(:to_i)
do_something if foo&.respond_to?(:to_s)
do_something if foo&.respond_to?(:to_f)
do_something if foo&.respond_to?(:to_h)

# AllowedMethods outside of conditions in assignment — no offense
result = foo&.is_a?(String)
x = foo&.eql?(bar)
y = foo&.equal?(baz)

# Parentheses around &.is_a? in || — RuboCop's check? sees parent as `begin`
# (parens), not `||`, so it is NOT flagged.
if @commentable.is_a?(Tag) || (@comment&.parent&.is_a?(Tag))
  do_something
end
