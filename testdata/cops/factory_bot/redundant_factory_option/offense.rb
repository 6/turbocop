association :user, factory: :user
                   ^^^^^^^^^^^^^^ FactoryBot/RedundantFactoryOption: Remove redundant `factory` option.
association :user, factory: %i[user]
                   ^^^^^^^^^^^^^^^^^ FactoryBot/RedundantFactoryOption: Remove redundant `factory` option.
association :user, :admin, factory: :user
                           ^^^^^^^^^^^^^^ FactoryBot/RedundantFactoryOption: Remove redundant `factory` option.
