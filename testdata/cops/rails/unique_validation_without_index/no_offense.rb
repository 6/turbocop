# Not a uniqueness validation
validates :account, length: { minimum: 5 }

# Uniqueness explicitly false
validates :account, uniqueness: false

# Presence validation
validates :name, presence: true

# Format validation
validates :email, format: { with: /\A[^@]+@[^@]+\z/ }

# Other validation types
validates :age, numericality: { greater_than: 0 }
