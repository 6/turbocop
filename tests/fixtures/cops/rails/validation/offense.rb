class User < ApplicationRecord
  validates_presence_of :name
  ^^^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, presence: true` instead of `validates_presence_of`.
  validates_uniqueness_of :email
  ^^^^^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, uniqueness: true` instead of `validates_uniqueness_of`.
  validates_comparison_of :age
  ^^^^^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, comparison: { ... }` instead of `validates_comparison_of`.
  validates_absence_of :archived
  ^^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, absence: true` instead of `validates_absence_of`.
  validates_format_of :phone
  ^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, format: { ... }` instead of `validates_format_of`.
  validates_numericality_of :score
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Validation: Use `validates :attr, numericality: true` instead of `validates_numericality_of`.
end
