class Foo
  def bar
  end

  private

  def baz
  end

  protected

  def qux
  end
end

# Access modifier right after class opening (no blank needed before)
class Bar
  private

  def secret
  end
end

# Access modifier right before end (no blank needed after)
class Baz
  def stuff
  end

  private
end

# Comment before modifier counts as separator
class Qux
  def stuff
  end

  # These methods are private
  private

  def secret
  end
end
