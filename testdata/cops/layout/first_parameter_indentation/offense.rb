def foo(
      bar,
      ^^^ Layout/FirstParameterIndentation: Use 2 (not 6) spaces for indentation.
  baz
)
end

def method_a(
        first,
        ^^^^^ Layout/FirstParameterIndentation: Use 2 (not 8) spaces for indentation.
  second
)
end

def method_b(
    first,
    ^^^^^ Layout/FirstParameterIndentation: Use 2 (not 4) spaces for indentation.
  second
)
end
