class Test
  def respond_to_missing?
  end

  def method_missing
  end
end

class Test2
  def self.respond_to_missing?
  end

  def self.method_missing
  end
end

class Test3
  private def respond_to_missing?
  end

  private def method_missing
  end
end

class Empty
end

class NoMethodMissing
  def foo
  end
end
