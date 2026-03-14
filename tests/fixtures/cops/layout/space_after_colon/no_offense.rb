{a: 1}
{b: "hello"}
{"c" => 2}
x = {foo: :bar}
y = {a: 1, b: 2, c: 3}
# Shorthand hash syntax (Ruby 3.1+): value omission
z = {url:, driver:}
# Colon followed by newline (hash value on next line)
h = {
  app_icon:
    APP_ICON_SIZES
}
# Optional keyword arguments with proper spacing
def f(a:, b: 2); end
def g(name: "default", size: 10); end
# Required keyword arguments (no value, no check needed)
def h(a:, b:); end
