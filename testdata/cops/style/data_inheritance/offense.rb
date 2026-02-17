class Person < Data.define(:first_name, :last_name)
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DataInheritance: Don't extend an instance initialized by `Data.define`. Use a block to customize the class.
  def age
    42
  end
end

class Person < ::Data.define(:first_name, :last_name)
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DataInheritance: Don't extend an instance initialized by `Data.define`. Use a block to customize the class.
  def age
    42
  end
end

class Person < Data.define(:first_name)
               ^^^^^^^^^^^^^^^^^^^^^^^^ Style/DataInheritance: Don't extend an instance initialized by `Data.define`. Use a block to customize the class.
end
