class A
  def self.three
  end
end

class B
  class << self
    attr_reader :two
  end
end

class C
  def self.foo
    42
  end
end
