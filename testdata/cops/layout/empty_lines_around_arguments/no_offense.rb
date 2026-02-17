# Good: no empty lines in multiline args
foo(
  bar,
  baz
)

# Good: single line call
something(first, second)

# Good: multiple args on separate lines
method_call(
  a,
  b,
  c
)

# Good: blank lines inside a hash argument value (not between arguments)
settings(index: index_preset, analysis: {
  filter: {
    english_stop: {
      type: 'stop',
    },

    english_stemmer: {
      type: 'stemmer',
    },
  },

  analyzer: {
    natural: {
      tokenizer: 'standard',
    },
  },
})

# Good: blank lines inside a block argument (block body is not between args)
Foo.prepend(Module.new do
  def something; end

  def anything; end
end)

# Good: blank line in heredoc argument
bar(<<-DOCS)
  foo

DOCS

# Good: blank line inside do..end block with no args that trails off
foo.baz do

  bar
end.compact

# Good: blank lines inside method body passed as block arg
result = @status.text.gsub(Account::MENTION_RE) do |match|
  username, domain = Regexp.last_match(1).split('@')

  domain = if TagManager.instance.local_domain?(domain)
             nil
           else
             TagManager.instance.normalize_domain(domain)
           end

  mentioned_account = Account.find_remote(username, domain)
end

# Good: receiver and method call on different lines
foo.

  bar(arg)

# Good: blank lines inside array argument
foo(:bar, [1,

           2]
)

# Good: empty lines inside a def argument
private(def bar

  baz
end)

# Good: multiline string with only whitespace on one line
format('%d

', 1)[0]

# Good: multiline style argument for method call without selector
foo.(
  arg
)

# Good: hash arg with blank lines between entries
ActiveRecord::Associations::Preloader.new(records: [@object], associations: {
  active_mentions: :account,

  reblog: {
    active_mentions: :account,
  },
}).call

# Good: method with argument that trails off heredoc
bar(<<-EOT)
  content

  more
EOT
  .call!(true)

# Good: empty line between normal arg & block arg's internal content is OK
Foo.prepend(
  a,
  Module.new do
    def something; end

    def anything; end
  end
)
