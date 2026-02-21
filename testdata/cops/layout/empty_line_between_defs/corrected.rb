class Foo
  def bar
    1
  end

  def baz
    2
  end

  def qux
    3
  end

  def quux
    4
  end

  def corge
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
    2
  end
  # first comment
  # second comment

  def charlie
    3
  end
  # inline comment on end

  def delta
    4
  end
end

# Too many blank lines between defs
class Garply
  def one
    1
  end


  def two
    2
  end



  def three
    3
  end
end
