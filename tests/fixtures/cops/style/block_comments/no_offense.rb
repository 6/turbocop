# Regular comment

# Multiple lines
# of comments

# Another comment
# here

x = 1

# =begin inside a heredoc should not be flagged
content = <<-CONTENT
=begin rdoc
some text
=end
    CONTENT

content2 = <<~HEREDOC
=begin
block comment lookalike
=end
HEREDOC

content3 = <<-'EOF'
=begin
  not a real block comment
=end
EOF

# =begin after __END__ should not be flagged (data section, not code)
__END__
=begin
this is data, not a block comment
=end
