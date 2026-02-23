module Foo

^ Layout/EmptyLinesAroundModuleBody: Extra empty line detected at module body beginning.
  def bar; end

^ Layout/EmptyLinesAroundModuleBody: Extra empty line detected at module body end.
end
module Bar

^ Layout/EmptyLinesAroundModuleBody: Extra empty line detected at module body beginning.
  X = 1
end
module Baz
  Y = 2

^ Layout/EmptyLinesAroundModuleBody: Extra empty line detected at module body end.
end
