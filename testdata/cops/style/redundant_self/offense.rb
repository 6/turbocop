def foo
  self.bar
  ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

def test
  self.to_s
  ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

def example
  self.method_name
  ^^^^ Style/RedundantSelf: Redundant `self` detected.
end

class Foo
  def self.name_for_response
    self.name.demodulize
    ^^^^ Style/RedundantSelf: Redundant `self` detected.
  end
end

class Bar
  def allowed(other)
    self.exists?(other)
    ^^^^ Style/RedundantSelf: Redundant `self` detected.
  end
end
