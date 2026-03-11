User.exists?(active: true)
User.where(active: true).count
User.exists?
Post.where(published: true).any?
User.find_by(active: true)
IpBlock.where(severity: :block).exists?(['ip >>= ?', remote_ip.to_s])

# where with a single string arg (SQL fragment) is NOT convertible to exists?
User.where('length(name) > 10').exists?
Post.where("published_at IS NOT NULL").exists?
Account.where("status != 'banned'").exists?

# where with string template + bindings (multiple args) IS convertible,
# but where with a single interpolated string is not
Record.where("#{table_name}.active = true").exists?

# where with a variable or method call arg is not convertible
Record.where(some_conditions).exists?
Record.where(build_query(params)).exists?
