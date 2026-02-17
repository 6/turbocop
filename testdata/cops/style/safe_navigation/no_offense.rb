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
