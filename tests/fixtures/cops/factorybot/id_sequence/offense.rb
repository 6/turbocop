FactoryBot.define do
  factory :post do
    sequence :id
    ^^^^^^^^^^^^ FactoryBot/IdSequence: Do not create a sequence for an id attribute
  end
end
FactoryBot.define do
  factory :user do
    sequence(:id, 1000)
    ^^^^^^^^^^^^^^^^^^^ FactoryBot/IdSequence: Do not create a sequence for an id attribute
  end
end
FactoryBot.define do
  factory :comment do
    sequence(:id, (1..10).cycle)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/IdSequence: Do not create a sequence for an id attribute
  end
end
