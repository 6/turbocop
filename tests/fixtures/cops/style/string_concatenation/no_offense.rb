"Hello #{name}"
"#{user.name} <#{user.email}>"
format('%s <%s>', user.name, user.email)
array.join(', ')
"foobar"
x = 'hello'

# Interpolated string on RHS with non-literal LHS — not flagged because
# neither side is a plain string literal (str_type? in RuboCop)
ENV.fetch('KEY') + "/#{path}"
account.username + "_#{i}"
pretty + "\n#{" " * nesting}}"
request.path + "?sort=#{field}&order=#{order}"

# Interpolated string on LHS with non-literal RHS
"#{index} " + user.email
rule_message + "\n#{explanation}"

# Multi-line heredoc content — in Parser these are dstr (not str_type?)
conf = @basic_conf + <<CONF
<match fluent.**>
  @type stdout
</match>
CONF

result = header + <<~HEREDOC
  some content here
  more content
HEREDOC

# Line-end concatenation (both sides str, + at end of line) — handled by Style/LineEndConcatenation
name = 'First' +
  'Last'

# Line-end concatenation with percent literals — handled by Style/LineEndConcatenation
"str" +
  %(str)

"str" +
  %q{str}

# Multi-line string literal — in Parser these are dstr (not str_type?)
# so RuboCop does not flag them. In Prism they are StringNode.
html = '
    <html>
        <head>
            <base href="' + base_url + '" />
        </head>
    </html>'

x = 'line1
line2' + y + 'line3
line4'

# Safe navigation &.+ is csend in Parser, not send — RuboCop ignores it
"jdbc:ch:#{CONNECTION_PARAMS[:protocol]&.+(':')}//host/db"

# Percent literal with backslash line continuation — both sides are str_type?
# in Parser, and + is at end of line → Style/LineEndConcatenation handles it
%(gem "test", "~> 1.0.0", ) +
  %(git: "https://example.com/\
test", tag: "v1.0.0")
