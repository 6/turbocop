a do ||
     ^ Style/EmptyBlockParameter: Omit pipes for the empty block parameters.
  do_something
end

a { || do_something }
    ^ Style/EmptyBlockParameter: Omit pipes for the empty block parameters.

[1, 2].each { || puts "hi" }
              ^ Style/EmptyBlockParameter: Omit pipes for the empty block parameters.
