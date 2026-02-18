FactoryBot.define do
  factory :post do
    title "Something"
    ^^^^^^^^^^^^^^^^^ FactoryBot/AttributeDefinedStatically: Use a block to declare attribute values.
    comments_count 0
    ^^^^^^^^^^^^^^^^ FactoryBot/AttributeDefinedStatically: Use a block to declare attribute values.
    tag Tag::MAGIC
    ^^^^^^^^^^^^^^ FactoryBot/AttributeDefinedStatically: Use a block to declare attribute values.
    recent_statuses []
    ^^^^^^^^^^^^^^^^^^ FactoryBot/AttributeDefinedStatically: Use a block to declare attribute values.
    published_at 1.day.from_now
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/AttributeDefinedStatically: Use a block to declare attribute values.
  end
end
