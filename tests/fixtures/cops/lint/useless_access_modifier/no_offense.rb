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

# define_method inside an each block — access modifier is not useless
class WithDefineMethodInIteration
  private

  [1, 2].each do |i|
    define_method("method#{i}") do
      i
    end
  end
end

# public after private, before a block that contains define_method
class WithDefineMethodInBlock
  private

  def some_private_method
  end

  public

  (CONFIGURABLE + NOT_CONFIGURABLE).each do |option|
    define_method(option) { @config[option] }
  end
end

# private before begin..end containing a method def
class WithBeginBlock
  private
  begin
    def method_in_begin
    end
  end
end
