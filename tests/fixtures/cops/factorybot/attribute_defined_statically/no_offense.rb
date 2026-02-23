FactoryBot.define do
  factory :post do
    trait :published do
      published_at { 1.day.from_now }
    end
    created_at { 1.day.ago }
    status { :draft }
    comments_count { 0 }
    title { "Static" }
    description { FFaker::Lorem.paragraph(10) }
    recent_statuses { [] }
    tags { { like_count: 2 } }
    before(:create, &:initialize_something)
    after(:create, &:rebuild_cache)
    sequence :negative_numbers, &:-@
    traits_for_enum :status, [:draft, :published]
    author age: 42, factory: :user
  end
end
