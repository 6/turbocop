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
