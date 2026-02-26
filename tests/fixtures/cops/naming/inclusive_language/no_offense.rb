allowlist_users = %w(admin)
denylist_ips = []
replica_count = 3
# Deploy to secondary node
primary_node = "node1"
follower_nodes = []
# Hash label syntax (tLABEL) is not checked by RuboCop
config = { whitelist: [], blacklist: [] }
# String content should not be flagged when CheckStrings is false (default)
puts "whitelist"
puts 'blacklist'
msg = "add to whitelist"
log("removed from blacklist")
error_msg = "the slave node is down"
