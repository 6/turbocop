allowlist_users = %w(admin)
denylist_ips = []
replica_count = 3
# Deploy to secondary node
primary_node = "node1"
follower_nodes = []
# Hash label syntax (tLABEL) is not checked by RuboCop
config = { whitelist: [], blacklist: [] }
