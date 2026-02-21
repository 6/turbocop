class User < ApplicationRecord
  validates :account, uniqueness: true
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.

  validates :username, uniqueness: { case_sensitive: false }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.

  validates_uniqueness_of :name
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.

  validates :account, uniqueness: { scope: :organization_id }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.
end
