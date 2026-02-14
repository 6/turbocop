class Foo
    x = 1
    ^^^ Layout/IndentationWidth: Use 2 (not 4) spaces for indentation.
end

def bar
 y = 2
 ^^^ Layout/IndentationWidth: Use 2 (not 1) spaces for indentation.
end

if true
      z = 3
      ^^^ Layout/IndentationWidth: Use 2 (not 6) spaces for indentation.
end
