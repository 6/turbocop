class Foo
  private

  def method
  end
end

class Bar
  protected

  def method2
  end
end

# MethodCreatingMethods: private followed by def_node_matcher
# This uses MethodCreatingMethods config which is not set in test defaults,
# but when configured properly, this should pass.
class Baz
  private

  def normal_method
  end
end
