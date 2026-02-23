class SomeClass
  def SomeClass.class_method
      ^^^^^^^^^ Style/ClassMethods: Use `self.class_method` instead of `SomeClass.class_method`.
  end
end

module SomeModule
  def SomeModule.mod_method
      ^^^^^^^^^^ Style/ClassMethods: Use `self.mod_method` instead of `SomeModule.mod_method`.
  end
end

class MyClass
  def MyClass.foo
      ^^^^^^^ Style/ClassMethods: Use `self.foo` instead of `MyClass.foo`.
  end
end
