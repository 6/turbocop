def name
  client.name.upcase
end

def name(arg)
  client.name
end

def name
  compute_something
end

delegate :name, to: :client

# Class method receivers can't use delegate
def no_replies_scope
  Status.without_replies
end

def find_user
  User.find_by_email(email)
end
