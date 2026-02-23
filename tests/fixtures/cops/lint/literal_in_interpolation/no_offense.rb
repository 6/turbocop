"result is #{compute}"
"hello #{name}"
"#{x + y}"
"#{foo(bar)}"
"no interpolation"
"#{variable}"

# Whitespace-only string literals in interpolation are deliberate
# (commonly used for trailing whitespace preservation in heredocs)
x = <<~MSG
  Add the following:#{' '}
MSG
"trailing space#{' '}"
"multi space#{'   '}"
