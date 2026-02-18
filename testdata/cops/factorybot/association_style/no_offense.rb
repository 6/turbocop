factory :article do
  user
end
factory :post do
  author factory: %i[user admin]
end
factory :user do
end
factory :article do
  association :user, strategy: :build
end
factory :article do
  author do
    association :user
  end
end
