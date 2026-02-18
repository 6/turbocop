do_something(<<~EOS)
  no string interpolation style text
EOS

do_something(<<~'EOS')
  text with #{interpolation} patterns
EOS

do_something(<<~`EOS`)
  command
EOS
x = 1
y = 2
