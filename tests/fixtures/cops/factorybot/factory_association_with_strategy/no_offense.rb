factory :foo do
  profile { association :profile }
end
factory :bar do
  profile
end
factory :baz do
  association :profile
end

# initialize_with blocks are procedural, not associations
factory :merchant_account, class: 'MerchantAccount' do
  user
  initialize_with do
    create(:agreement, user:)
    create(:compliance_info, user:)
    account = AccountManager.create_account(user)
    account
  end
end

# to_create blocks should not flag strategy calls
factory :readonly_record do
  to_create do |instance|
    build(:helper_record)
    instance.save(validate: false)
  end
end

# after callbacks should not flag strategy calls
factory :order do
  after(:create) do |order|
    create(:line_item, order: order)
  end
end

# before callbacks should not flag strategy calls
factory :team do
  before(:create) do |team|
    build(:member, team: team)
  end
end

# callback blocks should not flag strategy calls
factory :report do
  callback(:after_create) do |report|
    create(:attachment, report: report)
  end
end

# Multi-statement block bodies are procedural, not associations
factory :proposal do
  trait :with_evaluator do
    evaluator_role do
      space = component.participatory_space
      organization = space.organization
      build(:process_user_role, role: :evaluator, user: organization)
    end
  end
end

# Attribute named `build` with a literal value, not a strategy call
factory :host do
  trait :with_build do
    build { true }
  end
end
