def some_method; do_stuff
                 ^^^^^^^^ Style/TrailingBodyOnMethodDefinition: Place the first line of a multi-line method definition's body on its own line.
end

def f(x); b = foo
          ^^^^^^^ Style/TrailingBodyOnMethodDefinition: Place the first line of a multi-line method definition's body on its own line.
  b[c: x]
end

def bar; puts 'hello'
         ^^^^^^^^^^^^^ Style/TrailingBodyOnMethodDefinition: Place the first line of a multi-line method definition's body on its own line.
end
