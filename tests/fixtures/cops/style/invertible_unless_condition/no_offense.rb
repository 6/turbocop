if x != y
  do_something
end
do_something if x <= 0
unless condition
  do_something
end
if foo.odd?
  bar
end
unless complex_expression(a, b)
  bar
end
foo unless x != y || x.awesome?
foo unless x < Foo
foo unless x.present?
foo unless x.blank?
foo unless x.empty?
foo unless x.include?(y)
# Safe-navigation calls are not invertible (csend, not send)
foo unless x&.include?(y)
foo unless order&.any?
# Calls with blocks are not invertible (block node, not send)
foo unless items.any? { |i| i.valid? }
foo unless @queue.none? { |_, s, r| s == "spec" }
