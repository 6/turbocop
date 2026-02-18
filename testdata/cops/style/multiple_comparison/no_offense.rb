a = "a"
if a == "a"
  print a
end
["a", "b"].include?(a)
a == "x" || b == "y"
a == 1
x == y
foo.bar == "a" || foo.bar == "b"

# AllowMethodComparison (default: true) — method call values are allowed
x == foo.bar || x == baz.qux
username == config.local_domain || username == config.web_domain
