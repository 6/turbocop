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

# Lambda in body: RuboCop skips if any block/lambda exists in body
class College < ApplicationRecord
  with_options dependent: :destroy do |assoc|
    assoc.has_many :students, -> { where(active: true) }
  end
end

# Implicit receiver call wrapping param usage
with_options(opts) do |o|
  some_method(o.result)
end

# concat wrapping param usage
with_options wrapper_html: { class: ['extra'] } do |opt_builder|
  concat(opt_builder.input(:title, as: :string))
  concat(opt_builder.input(:author, as: :radio))
end

# Nested block in body
with_options options: false do |merger|
  merger.invoke
  with_another_method do |another_receiver|
    merger.invoke(another_receiver)
  end
end

# Non-param call in hash value argument (Devise.password_length)
with_options :if => :password_required? do |v|
  v.validates_presence_of     :password
  v.validates_confirmation_of :password
  v.validates_length_of       :password, :within => Devise.password_length, :allow_blank => true
end

# Lambda in hash argument — RuboCop skips blocks containing lambdas
with_options(:allow_blank => true) do |o|
  o.validates_numericality_of :var_value, only_integer: true, :if => -> { meta.var_type == :integer }
  o.validates_numericality_of :var_value,                     :if => -> { meta.var_type == :float   }
end

# Control flow (unless) inside with_options block — sends inside conditionals
# use non-param receivers, so RuboCop does not flag
with_options(association_options) do |m|
  m.has_many   :thumbnails, :class_name => "Thumb"
  unless reflect_on_association(:parent) || options[:thumbnails].empty?
    m.belongs_to :parent, :class_name => "Base", optional: true
  end
end
