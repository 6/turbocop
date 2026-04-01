x ? x : y
  ^ Style/RedundantCondition: Use double pipes `||` instead.

if a
^^ Style/RedundantCondition: Use double pipes `||` instead.
  a
else
  b
end

if foo
^^ Style/RedundantCondition: Use double pipes `||` instead.
  foo
else
  bar
end

x.nil? ? true : x
       ^ Style/RedundantCondition: Use double pipes `||` instead.

if a.empty?
^^ Style/RedundantCondition: Use double pipes `||` instead.
  true
else
  a
end

# unless with condition == else branch
unless b
^^^^^^ Style/RedundantCondition: Use double pipes `||` instead.
  y(x, z)
else
  b
end

# no-else pattern: if cond; cond; end → "This condition is not needed."
if do_something
^^ Style/RedundantCondition: This condition is not needed.
  do_something
end

# assignment branches: both branches assign to same variable
if foo
^^ Style/RedundantCondition: Use double pipes `||` instead.
  @value = foo
else
  @value = 'bar'
end

# local variable assignment branches
if foo
^^ Style/RedundantCondition: Use double pipes `||` instead.
  value = foo
else
  value = 'bar'
end

# method call branches with same receiver
if x
^^ Style/RedundantCondition: Use double pipes `||` instead.
  X.find(x)
else
  X.find(y)
end

# ternary with method call condition
b.x ? b.x : c
    ^ Style/RedundantCondition: Use double pipes `||` instead.

# ternary with function call condition
a = b(x) ? b(x) : c
         ^ Style/RedundantCondition: Use double pipes `||` instead.

# ternary predicate+true with number else
a.zero? ? true : 5
        ^ Style/RedundantCondition: Use double pipes `||` instead.

# constant path write assignment branches (FN fix)
if ENV['GIT_ADAPTER']
^^ Style/RedundantCondition: Use double pipes `||` instead.
  Gollum::GIT_ADAPTER = ENV['GIT_ADAPTER']
else
  Gollum::GIT_ADAPTER = 'rugged'
end

# method branches with operator receiver and multiline else expression
if volume
^^ Style/RedundantCondition: Use double pipes `||` instead.
  volumes << volume
else
  volumes << compute.volumes.create(
    name: volume_name,
    pool_name: compute.pools.first.name,
    capacity: 1
  )
end

# multiline ternary with line continuation
refs = (self.roxml_references \
  ? self.roxml_references \
  ^ Style/RedundantCondition: Use double pipes `||` instead.
  : self.class.roxml_attrs.map { |attr| attr.to_ref(self) })

# predicate+true with multiline else call
if APPSIGNAL_AGENT_CONFIG["triples"].key?(TARGET_TRIPLE)
^^ Style/RedundantCondition: Use double pipes `||` instead.
  true
else
  abort_installation(
    "AppSignal currently does not support your system architecture (#{TARGET_TRIPLE})." \
      "Please let us know at support@appsignal.com, we aim to support everything " \
      "our customers run."
  )
end

# method branches where else argument is a lambda expression
if implementation
^^ Style/RedundantCondition: Use double pipes `||` instead.
  install_method_callback implementation
else
  install_method_callback(lambda do |*lambda_args|
    args.first
  end)
end

# predicate+true inside an assignment with a multiline else expression
is_nullable =
  if spectrum?(table)
  ^^ Style/RedundantCondition: Use double pipes `||` instead.
    true
  else
    case nullable
    when 'YES'
      true
    else
      false
    end
  end

# condition matches the if branch, else branch is a multiline call
if type
^^ Style/RedundantCondition: Use double pipes `||` instead.
  type
else
  get_call_expr_type(
    Call.make(name: ast.name, parent: ast.parent),
    type_env,
    ast.name
  )
end

# unless assignment branches compare against the syntactic else branch
unless option
^^^^^^ Style/RedundantCondition: Use double pipes `||` instead.
  @print_headers = 1
else
  @print_headers = option
end

# condition matches the if branch, else branch is a block call
if account
^^ Style/RedundantCondition: Use double pipes `||` instead.
  account
else
  Account.create! do |a|
    a.name = account_name
  end
end

# multiline ternary with line continuation in a parenthesized assignment
refs = (self.roxml_references \
  ? self.roxml_references \
  ^ Style/RedundantCondition: Use double pipes `||` instead.
  : fallback_refs)

# predicate+true with block-pass predicate
if futures.all?(&:fulfilled?)
^^ Style/RedundantCondition: Use double pipes `||` instead.
  true
else
  false
end
