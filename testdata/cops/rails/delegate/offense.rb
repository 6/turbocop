def name
^^^ Rails/Delegate: Use `delegate` to define delegations.
  client.name
end

def email
^^^ Rails/Delegate: Use `delegate` to define delegations.
  account.email
end

def title
^^^ Rails/Delegate: Use `delegate` to define delegations.
  post.title
end

def site_title
^^^ Rails/Delegate: Use `delegate` to define delegations.
  Setting.site_title
end

def [](key)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  @attrs[key]
end

def []=(key, value)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  @attrs[key] = value
end

def fetch(arg)
^^^ Rails/Delegate: Use `delegate` to define delegations.
  client.fetch(arg)
end
