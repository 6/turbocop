delegate :name, to: :client, allow_nil: true
delegate :name, to: :client
delegate :email, :phone, to: :account
delegate :title, to: :post, prefix: true
delegate :address, to: :user
