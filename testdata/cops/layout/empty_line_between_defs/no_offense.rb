class Foo
  def bar
    1
  end

  def baz
    2
  end

  # comment for qux
  def qux
    3
  end
end

# do..end block before definition — no blank line required
class Bar
  items.each do |item|
    process(item)
  end
  def foo
    1
  end

  def bar
    2
  end
end

# if..end before definition — no blank line required
class Baz
  if condition
    setup
  end
  def foo
    1
  end

  def bar
    2
  end
end

# begin..end before definition — no blank line required
class Qux
  begin
    setup
  end
  def foo
    1
  end
end

# Two defs separated by blank line + comments — no offense
class Quux
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
end

# Adjacent single-line defs are allowed (AllowAdjacentOneLineDefs: true)
class Corge
  def alpha; 1 end
  def bravo; 2 end
  def charlie; 3 end
end

# Single-line def after multi-line def with blank line — no offense
class Grault
  def foo
    1
  end

  def bar; 2 end
end
