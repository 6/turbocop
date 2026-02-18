factory :foo, class: Foo do
                     ^^^ FactoryBot/FactoryClassName: Pass 'Foo' string instead of `Foo` constant.
end
factory :bar, class: Foo::Bar do
                     ^^^^^^^^ FactoryBot/FactoryClassName: Pass 'Foo::Bar' string instead of `Foo::Bar` constant.
end
factory :baz, class: ::Foo do
                     ^^^^^ FactoryBot/FactoryClassName: Pass '::Foo' string instead of `::Foo` constant.
end
