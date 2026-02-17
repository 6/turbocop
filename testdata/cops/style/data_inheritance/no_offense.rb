class Person
end

Person = Data.define(:first_name, :last_name)

Person = Data.define(:first_name, :last_name) do
  def age
    42
  end
end

class Person < ActiveRecord::Base
end
