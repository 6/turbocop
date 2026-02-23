def foo(x=1)
         ^ Layout/SpaceAroundEqualsInParameterDefault: Surrounding space missing for operator `=`.
end
def bar(a=1, b=2)
         ^ Layout/SpaceAroundEqualsInParameterDefault: Surrounding space missing for operator `=`.
              ^ Layout/SpaceAroundEqualsInParameterDefault: Surrounding space missing for operator `=`.
end
def baz(x ="hello")
          ^ Layout/SpaceAroundEqualsInParameterDefault: Surrounding space missing for operator `=`.
end
