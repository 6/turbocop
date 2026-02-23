factory :foo, class: 'Foo' do
end
factory :bar, class: Hash do
end
factory :baz, class: OpenStruct do
end
factory :qux, class: 'Foo::Bar' do
end
factory :quux, class: 'Foo'
