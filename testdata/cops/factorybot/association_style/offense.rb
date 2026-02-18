factory :article do
  association :user
  ^^^^^^^^^^^^^^^^^ FactoryBot/AssociationStyle: Use implicit style to define associations.
end
factory :post do
  association :author, factory: :user
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/AssociationStyle: Use implicit style to define associations.
end
factory :comment do
  trait :with_user do
    association :user
    ^^^^^^^^^^^^^^^^^ FactoryBot/AssociationStyle: Use implicit style to define associations.
  end
end
