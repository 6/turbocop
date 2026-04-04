# Placeholder: Lint/Syntax errors are reported by the parser (Prism),
# not by this cop. This cop exists for configuration compatibility.
# Actual offense detection is tested via lint_source_inner in unit tests.
x = 1
y = 2
z = 3

retry
^ Lint/Syntax: Invalid retry without rescue

retry
^ Lint/Syntax: Invalid retry without rescue

class Foo
  return 1
  ^^^^^^ Lint/Syntax: Invalid return in class/module body
end
