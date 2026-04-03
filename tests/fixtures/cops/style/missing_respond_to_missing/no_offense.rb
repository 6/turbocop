def method_missing(message, *args, &block)
  return get(Regexp.last_match(1).to_sym, *args, &block) if message.to_s.match?(/^get_(.*)/)
  super
end

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

Class.new do
  def respond_to_missing?
  end

  def method_missing
  end
end

# Keep only the unstable top-level forms here: RuboCop still reports many
# explicit-arg top-level `method_missing` defs, but file-leading block-arg
# forms and zero-arg/rest-only signatures are inconsistent.
def respond_to_missing?
end

def method_missing
end

def method_missing(*args)
end
