x = '%{name} is %{age}'
    ^^^^^^^^^^^^^^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
y = format('%s %s %d', a, b, c)
           ^^^^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
z = '%{greeting} %{target}'
    ^^^^^^^^^^^^^^^^^^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over template tokens (like `%{foo}`).
w = sprintf('%s %s', a, b)
            ^^^^^^^^ Style/FormatStringToken: Prefer annotated tokens (like `%<foo>s`) over unannotated tokens (like `%s`).
