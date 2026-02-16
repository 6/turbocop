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
