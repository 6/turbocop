class Foo
^^^^^ Lint/EmptyClass: Empty class detected.
end

class Bar < Base
^^^^^ Lint/EmptyClass: Empty class detected.
end

class Baz
^^^^^ Lint/EmptyClass: Empty class detected.
  # just a comment (AllowComments default false)
end
