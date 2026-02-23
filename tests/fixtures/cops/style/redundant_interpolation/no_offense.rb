"hello #{name}"

"#{x} world"

"foo"

x.to_s

"hello"

# Implicit string concatenation with backslash — not redundant
"#{foo}" \
  "bar"

"#{foo}" \
  "bar #{baz}"

"hello " \
  "#{world}"

# Implicit concatenation with multiple interpolated parts
"#{AGENT_INSTRUCTION} Please rephrase the following response. " \
  "#{LANGUAGE_INSTRUCTION}"
