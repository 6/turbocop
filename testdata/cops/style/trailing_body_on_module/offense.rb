module Foo; extend self
            ^^^^^^^^^^^ Style/TrailingBodyOnModule: Place the first line of module body on its own line.
end

module Bar; include Baz
            ^^^^^^^^^^^ Style/TrailingBodyOnModule: Place the first line of module body on its own line.
end

module Qux; def foo; end
            ^^^^^^^^^^^^^^ Style/TrailingBodyOnModule: Place the first line of module body on its own line.
end
