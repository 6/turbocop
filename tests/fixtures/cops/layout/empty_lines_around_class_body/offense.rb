class Foo

^ Layout/EmptyLinesAroundClassBody: Extra empty line detected at class body beginning.
  def bar; end

^ Layout/EmptyLinesAroundClassBody: Extra empty line detected at class body end.
end
class Bar

^ Layout/EmptyLinesAroundClassBody: Extra empty line detected at class body beginning.
  X = 1
end
class Baz
  Y = 2

^ Layout/EmptyLinesAroundClassBody: Extra empty line detected at class body end.
end
class << self

^ Layout/EmptyLinesAroundClassBody: Extra empty line detected at class body beginning.
  def foo
  end

^ Layout/EmptyLinesAroundClassBody: Extra empty line detected at class body end.
end
class Qux
  class << self

^ Layout/EmptyLinesAroundClassBody: Extra empty line detected at class body beginning.
    def bar
    end
  end
end
