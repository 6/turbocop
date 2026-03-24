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

# Method definitions with ?/! suffixes are tIDENTIFIER in parser gem (checked)
def whitelisted?
    ^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `whitelist`. Suggested alternatives: `allowlist`, `permit`.
  true
end

def blacklisted?(value)
    ^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `blacklist`. Suggested alternatives: `denylist`, `block`.
  false
end

def _makara_blacklist!
            ^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `blacklist`. Suggested alternatives: `denylist`, `block`.
  disconnect!
end

# Bare symbols inside string interpolation are tSYMBOL in parser gem (CheckSymbols: true)
opts << "dsn=#{@opts[:slave_monitor_dsn]}"
                      ^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `slave`. Suggested alternatives: `replica`, `secondary`, `follower`.

# undef arguments are tIDENTIFIER in parser gem (CheckIdentifiers: true)
undef new_slave, new_safe_slave
      ^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `slave`. Suggested alternatives: `replica`, `secondary`, `follower`.
                 ^^^^^^^^^^^^^^ Naming/InclusiveLanguage: Use inclusive language instead of `slave`. Suggested alternatives: `replica`, `secondary`, `follower`.
