x = :true
    ^^^^^ Lint/BooleanSymbol: Symbol with a boolean name - you probably meant to use `true`.
y = :false
    ^^^^^^ Lint/BooleanSymbol: Symbol with a boolean name - you probably meant to use `false`.
z = :true
    ^^^^^ Lint/BooleanSymbol: Symbol with a boolean name - you probably meant to use `true`.

options = { true: "yes", false: "no" }
            ^^^^^ Lint/BooleanSymbol: Symbol with a boolean name - you probably meant to use `true`.
                         ^^^^^^ Lint/BooleanSymbol: Symbol with a boolean name - you probably meant to use `false`.
