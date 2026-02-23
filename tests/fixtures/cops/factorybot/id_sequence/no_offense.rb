FactoryBot.define do
  factory :post do
    summary { "A summary" }
    sequence :something_else
    title { "A title" }
  end
end
FactoryBot.define do
  sequence(id)
end
FactoryBot.define do
  sequence
end
FactoryBot.define do
  foo.sequence(:id)
end
