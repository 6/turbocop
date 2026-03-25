foo && bar
foo.bar && foo.baz
foo.bar || foo.baz
foo&.bar || foobar.baz
foo&.bar || foo.nil?
foo.nil? && foo&.bar
foo&.bar && foo.baz
foo&.bar && foo == 1
foo&.bar || foo == 1
foo&.bar && foo != 1
foo&.bar || foo != 1
foo&.bar && foo === 1
foo&.bar || foo === 1
foo == bar || foo == baz
return true if a.nil? || a&.whatever?
foo&.bar && foobar.baz && foo.qux
foobar.baz && foo&.bar && foo.qux
foobar.baz && foo&.bar && foo.qux && foo.foobar
foo&.bar || foo&.baz && foo.qux
foo&.bar && (foo.baz || foo.qux)
(foo&.bar && foo.baz) || foo.qux
foo&.bar && foo.baz = 1
foo&.bar && foo + 1
foo&.bar && foo - 1
foo&.bar && foo << 1
x.foo.bar && y.foo&.baz

# Separate && branches within an || chain should not be merged together
(foo&.a && x) || (foo&.b && y) || (foo&.c && z)
foo&.a || foo&.b || (foo&.c && flag) || foo&.d

# Block-wrapped calls should not be collected as operands (RuboCop skips block nodes)
setting&.domains&.include?(domain) || setting&.domains.any? { |d| d.match?(/x/) }
foo&.bar || foo.baz { |x| x }
foo&.bar && foo.baz { |x| x }

# Parenthesized && groups inside || — RuboCop does not recurse into begin nodes
(klass_or_obj&.is_a?(Class) && klass_or_obj < ActiveRecord::Base) || (klass_or_obj&.is_a?(ActiveRecord::Base) && klass_or_obj.new_record?)
(foo&.a && bar) || (foo&.b && baz)
foo&.bar && (foobar.baz || foo&.baz)

# Operator methods called with &. — operator_method? is true so already_appropriate for "."
floated_corporation_index&.positive? && floated_corporation_index&.<(order.size)
foo&.bar && foo&.+(1)
foo&.bar && foo&.>=(threshold)
