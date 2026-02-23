do_something(<<~EOS)
  no string interpolation style text
EOS

do_something(<<~'EOS')
  text with #{interpolation} patterns
EOS

do_something(<<~`EOS`)
  command
EOS

do_something(<<~'EOS')
  text with #@instance_var patterns
EOS

do_something(<<~'EOS')
  text with #$global_var patterns
EOS

do_something(<<~'EOS')
  Preserve \
  newlines
EOS

do_something(<<~"EDGE'CASE")
  no string interpolation style text
EDGE'CASE

x = 1
y = 2
