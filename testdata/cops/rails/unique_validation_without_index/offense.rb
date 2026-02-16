validates :account, uniqueness: true
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.

validates :email, uniqueness: { case_sensitive: false }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.

validates_uniqueness_of :username
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.

validates :name, uniqueness: { scope: :organization_id }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/UniqueValidationWithoutIndex: Uniqueness validation should have a unique index on the database column.
