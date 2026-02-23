class Person < Struct.new(:first_name, :last_name)
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StructInheritance: Don't extend an instance initialized by `Struct.new`. Use a block to customize the struct.
  def foo; end
end

class Person < ::Struct.new(:first_name, :last_name)
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/StructInheritance: Don't extend an instance initialized by `Struct.new`. Use a block to customize the struct.
  def foo; end
end

class Person < Struct.new(:first_name)
               ^^^^^^^^^^^^^^^^^^^^^^^ Style/StructInheritance: Don't extend an instance initialized by `Struct.new`. Use a block to customize the struct.
end
