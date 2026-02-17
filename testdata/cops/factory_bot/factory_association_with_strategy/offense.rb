factory :foo, class: 'FOOO' do
  profile { create(:profile) }
            ^^^^^^^^^^^^^^^^ FactoryBot/FactoryAssociationWithStrategy: Use an implicit, explicit or inline definition instead of hard coding a strategy for setting association within factory.
end
factory :bar do
  profile { build(:profile) }
            ^^^^^^^^^^^^^^^ FactoryBot/FactoryAssociationWithStrategy: Use an implicit, explicit or inline definition instead of hard coding a strategy for setting association within factory.
end
factory :baz do
  profile { build_stubbed(:profile) }
            ^^^^^^^^^^^^^^^^^^^^^^^ FactoryBot/FactoryAssociationWithStrategy: Use an implicit, explicit or inline definition instead of hard coding a strategy for setting association within factory.
end
