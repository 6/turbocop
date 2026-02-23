x = a.to_f / b.to_f
    ^^^^^^^^^^^^^^^ Style/FloatDivision: Prefer using `.to_f` on one side only.
y = (a - b).to_f / (3 * d / 2).to_f
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/FloatDivision: Prefer using `.to_f` on one side only.
z = x.to_f / y.to_f
    ^^^^^^^^^^^^^^^^ Style/FloatDivision: Prefer using `.to_f` on one side only.
