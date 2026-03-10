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
warn "[DEPRECATION] `:whitelist` mode is deprecated."
warn "replace :blacklist with :denylist"
# Heredoc content should not be flagged (CheckStrings: false by default)
description = <<~TEXT
  The whitelist feature allows blocking specific IPs.
  Items on the blacklist are automatically rejected.
TEXT
# %i array content should not be flagged
ATTRS = %i[
  allowed_emails
  blocked_domains
  whitelisted_items
  blacklisted_names
]
# %w array content should not be flagged
NAMES = %w[
  whitelisted_emails
  blacklisted_domains
]
# tFID tokens (identifiers ending in ! or ?) are not checked by RuboCop
blacklisted?
whitelist!
_makara_blacklist!
# Instance method defs with ?/! (def foo?) are tIDENTIFIER in parser gem (checked),
# so they are in offense.rb. Calls and singleton defs (def self.foo?) are tFID (not checked).
# Quoted symbols are treated as string content by RuboCop's parser (tSTRING_CONTENT)
# so they follow CheckStrings (false by default), not CheckSymbols
x = :"errors.messages.content_type_whitelist_error"
y = :'content_type_blacklist_error'
validate_config(
  whitelisted_emails: [],
  blacklisted_domains: [],
  whitelist_enabled: true,
  blacklist_duration: 30
)
# String literals inside string interpolation follow CheckStrings (false), not code
settings = "'#{[AppSettings['settings.extension_whitelist'].split(',')].join("', '")}'".html_safe
options = "modprobe.blacklist=#{params['blacklist'].delete(' ')}"
# Method calls ending in ?/! are tFID in parser gem — not checked
whitelisted?
blacklisted?
_makara_blacklist!
obj.whitelisted?
# Singleton method definitions (def self.xxx?) are also tFID — not checked
def self.is_whitelisted?(url)
  true
end
def self.blacklisted?(key)
  false
end
def self.is_blacklisted?(email)
  true
end
