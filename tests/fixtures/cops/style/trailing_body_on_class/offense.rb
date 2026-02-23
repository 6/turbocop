class Foo; def foo; end
           ^^^^^^^^^^^^^^ Style/TrailingBodyOnClass: Place the first line of class body on its own line.
end

class Bar; bar = 1
           ^^^^^^^ Style/TrailingBodyOnClass: Place the first line of class body on its own line.
end

class Baz < Base; include Mod
                  ^^^^^^^^^^^ Style/TrailingBodyOnClass: Place the first line of class body on its own line.
end
