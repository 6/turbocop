x = "hello"
y = 'world'
z = <<~TEXT
  heredoc content
TEXT
a = "double \"quoted\""
b = 'single \'quoted\''
rule %r/"""/, Str, :tdqs
rule %r/'''/, Str, :tsqs
x = /"""/
y = /'''/
rule %r/r""".*?"""/m, Str::Other
rule %r/r'''.*?'''/m, Str::Other
# Triple quotes inside string arguments should not be flagged
is_block = str.start_with?('"""')
BLOCK_QUOTE = '"""'
y = "contains \"\"\" inside"
tokens = subject.tokenize('""""""""""')
x = str.end_with?("'''")
