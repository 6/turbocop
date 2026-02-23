User.exists?(active: true)
User.where(active: true).count
User.exists?
Post.where(published: true).any?
User.find_by(active: true)
IpBlock.where(severity: :block).exists?(['ip >>= ?', remote_ip.to_s])
