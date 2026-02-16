def foo
  # aligned with next line
  x = 1
  # also aligned
  y = 2
end
# top level comment
z = 3

# Comment before else can match body indentation
if true
  x = 1
  # comment about else branch
else
  y = 2
end

# Comment before else can match keyword indentation
if true
  x = 1
# comment about else
else
  y = 2
end

# Comment before end should align with body
def bar
  x = 1
  # closing comment
end

# Comment before when can match body
case x
when 1
  a = 1
  # about next case
when 2
  b = 2
end

# Comment before rescue
begin
  risky
  # rescue comment
rescue => e
  handle(e)
end

# Lines starting with # inside heredocs are NOT comments
environment <<~end_of_config, env: "production"
  # Prepare the ingress controller used to receive mail
  # config.action_mailbox.ingress = :relay
end_of_config

x = <<~HEREDOC
  # this looks like a comment but is string content
  # and should not be checked for indentation
HEREDOC

y = <<-SQL
# also inside a heredoc
  # with weird indentation
SQL

# `#` inside regex with /x extended mode is a regex comment, not Ruby
path = build_path(
  "/page/:name",
  { name: /
    #ROFL
    (tender|love
    #MAO
    )/x },
  true
)

# `#` inside multi-line string (interpolation) is not a comment
system "bundle exec dartsass \
  #{@guides_dir}/assets/stylesrc/style.scss:#{@output_dir}/style.css \
  #{@guides_dir}/assets/stylesrc/highlight.scss:#{@output_dir}/highlight.css \
  #{@guides_dir}/assets/stylesrc/print.scss:#{@output_dir}/print.css"
