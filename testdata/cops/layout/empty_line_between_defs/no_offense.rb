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
