class Account < ApplicationRecord
  with_options dependent: :destroy do |assoc|
    assoc.has_many :customers
    ^^^^^ Rails/RedundantReceiverInWithOptions: Redundant receiver in `with_options`.
    assoc.has_many :products
    ^^^^^ Rails/RedundantReceiverInWithOptions: Redundant receiver in `with_options`.
    assoc.has_many :invoices
    ^^^^^ Rails/RedundantReceiverInWithOptions: Redundant receiver in `with_options`.
  end
end
