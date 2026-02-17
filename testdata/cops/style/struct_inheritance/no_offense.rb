class Person
end

Person = Struct.new(:first_name, :last_name)

Person = Struct.new(:first_name, :last_name) do
  def age
    42
  end
end

class Person < DelegateClass(Animal)
end
