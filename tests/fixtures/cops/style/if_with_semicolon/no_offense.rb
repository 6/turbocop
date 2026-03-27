if foo
  bar
end

foo ? bar : baz

bar if foo

if foo
  bar
else
  baz
end

# Multi-line if with comment containing semicolons should not be flagged
if quantifier1 == quantifier2
  # (?:a+)+ equals (?:a+) ; (?:a*)* equals (?:a*)
  quantifier1
else
  '*'
end

# Multi-line if with semicolons in comment between condition and body
if provider == 'whatsapp_cloud'
  # The callback is for manual setup flow; embedded signup handles it
  default_config = {}
end

# else if pattern: inner if has parent that is if_type, RuboCop skips it
if x > 0
  foo
else if y > 0; bar else baz end
end

# Nested if inside else branch (parent is if_type)
if a
  something
else if b; c end
end

# Multi-line if with comment containing semicolon after condition (FP fix)
if (spec_override = status["replicas"].presence) # ignores possibility of surge; need a spec_replicas arg for that
  result["spec"]["replicas"] = spec_override
end

# Simple if with comment containing semicolon
if condition # this is a comment; with semicolon
  do_something
end

# Unless with comment containing semicolon
unless done # not done; keep going
  process
end

# If with semicolon as sole child of another if's branch (RuboCop: node.parent&.if_type?)
# In parser gem, sole child's parent IS the if node, so if_type? is true → skip
if outer_cond
  if inner_cond; foo
  else; bar; end
end

# If with semicolon as sole child of elsif branch
if cond1
  foo
elsif cond2
  if inner; bar
  else; baz; end
end

# If with semicolon as sole child inside nested if (deeper nesting)
if Mouse.button_released?
  if @anchor1
    if @cur_node != @anchor1; @anchor2 = @cur_node
    else; @anchor1 = nil; end
  end
end
