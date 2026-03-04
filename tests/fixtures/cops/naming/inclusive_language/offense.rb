whitelist_users = %w(admin)
^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `whitelist`. Suggested alternatives: `allowlist`, `permit`.

blacklist_ips = []
^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `blacklist`. Suggested alternatives: `denylist`, `block`.

# Remove slave nodes from cluster
         ^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `slave`. Suggested alternatives: `replica`, `secondary`, `follower`.

msg = "connected to #{slave_host}"
                      ^^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `slave`. Suggested alternatives: `replica`, `secondary`, `follower`.

# Symbol literals should be flagged (CheckSymbols: true by default)
config[:whitelist] = []
        ^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `whitelist`. Suggested alternatives: `allowlist`, `permit`.

def whitelisted?
    ^^^^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `whitelist`. Suggested alternatives: `allowlist`, `permit`.
  true
end

def blacklisted?
    ^^^^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `blacklist`. Suggested alternatives: `denylist`, `block`.
  false
end

alias allowlist= whitelist=
                 ^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `whitelist`. Suggested alternatives: `allowlist`, `permit`.

alias blocklist= blacklist=
                 ^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `blacklist`. Suggested alternatives: `denylist`, `block`.

query = "grant select to '#{env.db_slave_user}'"
                                   ^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `slave`. Suggested alternatives: `replica`, `secondary`, `follower`.

script = <<~SQL
  grant select on *.* to '#{env.db_slave_user}'@'%'
                                   ^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `slave`. Suggested alternatives: `replica`, `secondary`, `follower`.
SQL
