class Foo
  def bar
    1
  end
  def baz
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    2
  end
  def qux
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    3
  end

  def quux
    4
  end
  def corge
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    5
  end
end

# Two defs separated only by comments (no blank lines)
class Grault
  def alpha
    1
  end
  # comment about bravo
  def bravo
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    2
  end
  # first comment
  # second comment
  def charlie
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    3
  end
  # inline comment on end
  def delta
  ^^^ Layout/EmptyLineBetweenDefs: Use empty lines between method definitions.
    4
  end
end

# Too many blank lines between defs
class Garply
  def one
    1
  end


  def two
  ^^^ Layout/EmptyLineBetweenDefs: Expected 1 empty line between method definitions; found 2.
    2
  end



  def three
  ^^^ Layout/EmptyLineBetweenDefs: Expected 1 empty line between method definitions; found 3.
    3
  end
end
