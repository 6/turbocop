Struct.new(:name)
MyStruct = Struct.new(:name, :age)
Hash.new
{name: "test"}
x = Struct.new(:foo, :bar)
::Struct.new(:name)
# Bare OpenStruct references (not .new calls) should not be flagged
factory :email, class: OpenStruct do
end
x.is_a?(OpenStruct)
klass = OpenStruct
items = [OpenStruct, Hash]
rescue OpenStruct => e
