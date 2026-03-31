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

class Test4
  class << self
    def respond_to_missing?
    end

    def method_missing
    end
  end
end

class Test5
  def respond_to_missing?
  end

  if condition
    def method_missing
    end
  end
end

class Test6
  def respond_to_missing?
  end

  class_eval do
    def method_missing
    end
  end
end

module Test7
  def method_missing
  end

  class Inner
    def respond_to_missing?
    end
  end
end
