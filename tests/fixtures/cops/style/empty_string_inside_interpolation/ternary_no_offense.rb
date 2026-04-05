# nitrocop-config: EnforcedStyle: ternary
"#{condition ? 'foo' : ''}"

%(<nav #{
  data = []
  data.push(:x)
  data.join
}>)

%(<nav #{
  if keynav?
    'x'
  else
    'y'
  end
}>)
