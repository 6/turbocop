class User < ApplicationRecord
  attribute :tags, default: -> { [] }
  attribute :active, default: true
  attribute :role, default: :member
  attribute :login_count, :integer, default: 0
  attribute :notes, default: "pending"
  attribute :status, :string, default: "active"
  # lambda keyword call already wraps value in a block — not flagged by RuboCop
  attribute :feature_flag, :boolean, default: lambda {
    Settings.enabled?(:feature_flag)
  }
  attribute :config, default: proc { { key: "value" } }
end
