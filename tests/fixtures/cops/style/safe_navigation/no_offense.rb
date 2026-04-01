foo&.bar
foo&.bar&.baz
foo && foo.owner.nil?
foo && foo.empty?
foo && bar.baz
foo && foo < bar

# Dotless operator calls ([], []=, +, etc.) — safe nav not idiomatic
previous && previous['verified_at'].present?
obj && obj[:key].method_call
options && options[:codecs].include?(codec)
foo && foo[0].bar
foo && foo + bar

# Ternary with [] operator — not idiomatic with safe nav
foo ? foo[index] : nil
foo ? foo[idx] = v : nil

# Ternary with nil? result (not safe nav pattern)
foo.nil? ? bar : baz

# Ternary with empty? — unsafe
foo.nil? ? nil : foo.empty?

# Ternary: foo ? nil : foo.bar — wrong direction
foo ? nil : foo.bar

# Methods that nil responds to in the chain — unsafe to convert
foo && foo.owner.is_a?(SomeClass)
foo && foo.value.respond_to?(:call)
foo && foo.name.kind_of?(String)

# AllowedMethods (present?, blank?) in the chain
config && config.value.present?
foo && foo.bar.blank?
portal && portal.custom_domain.present?

# && inside assignment method call (e.g. []=) — unsafe context
cookies[token] = user && user.remember_me!
result[key] = obj && obj.value
foo.bar = baz && baz.qux

# && inside dotless method call arguments — unsafe context
# (RuboCop skips when ancestor send is dotless, e.g. scope, puts)
scope :accessible_to_user, ->(user) { user && user.name }
puts(foo && foo.bar)

# Negated wrappers make safe navigation unsafe
!!(foo && foo.bar)
obj.do_something if !obj

# Outer operator/assignment parents make modifier `if` unsafe
value - begin
  foo.bar if foo
end - used

hash[:categories] = begin
  foo.bar if foo
end

# && inside send/public_send arguments — RuboCop skips dynamic dispatch context
obj.send(:x, foo && foo.map { |h| h })
obj.public_send(:x, foo && foo.downcase)
