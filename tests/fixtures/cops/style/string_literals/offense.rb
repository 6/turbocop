x = "hello"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
y = "world"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
z = "foo bar"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
u = "has \\ backslash"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
a = "\\"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
b = "\""
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
c = "España"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
# Strings with only \" escapes can use single quotes (\" becomes literal " in single quotes)
d = "execve(\"/bin/sh\", rsp, environ)"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
e = "{\"key\": \"value\"}"
    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.

changes = `git rev-list v#{ENV["PREVIOUS_VERSION"]}..HEAD | bundle exec github_fast_changelog AlchemyCMS/alchemy_cms`.split("\n")
                               ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.

`bundle binstub vite_ruby --path #{config.root.join("bin")}`
                                                    ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.

`lua \
/usr/local/openresty/nginx/count-von-count/lib/log_player.lua \
/usr/local/openresty/nginx/logs/access.log \
#{spec_config["redis_host"]} \
              ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
#{spec_config["redis_port"]} \
              ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
#{spec_config["log_player_redis_db"]} \
              ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
`

`#{command.join(" ")}`
                ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.

`#{taylor("squash --stdout")}`
          ^ Style/StringLiterals: Prefer single-quoted strings when you don't need string interpolation or special symbols.
