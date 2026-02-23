class Account < ApplicationRecord
  with_options dependent: :destroy do
    has_many :customers
    has_many :products
    has_many :invoices
    has_many :expenses
  end
end

with_options options: false do |merger|
end

# Mixed receivers: not all sends use block param
with_options instance_writer: false do |serializer|
  serializer.class_attribute :_named_contexts
  serializer.class_attribute :_context_extensions
  self._named_contexts     ||= {}
  self._context_extensions ||= {}
end
