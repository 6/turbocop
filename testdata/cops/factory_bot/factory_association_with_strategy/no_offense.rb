factory :foo do
  profile { association :profile }
end
factory :bar do
  profile
end
factory :baz do
  association :profile
end
