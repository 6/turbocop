association :user
association :user, strategy: :build
association :author, factory: :user
association :user, factory: %i[user admin]
association :profile, factory: :profile_v2
