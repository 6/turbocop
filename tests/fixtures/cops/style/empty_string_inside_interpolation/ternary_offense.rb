# nitrocop-config: EnforcedStyle: ternary
%(<a href="#"#{
             ^^ Style/EmptyStringInsideInterpolation: Do not use trailing conditionals in string interpolation.
  %( #{anchor_string}) if anchor_string
})

%(<nav #{
       ^^ Style/EmptyStringInsideInterpolation: Do not use trailing conditionals in string interpolation.
  data = []
  data.push(:x) if keynav?
  data.join
}>)
