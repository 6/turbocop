foo&.bar
foo&.bar&.baz
foo && foo.nil?
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
