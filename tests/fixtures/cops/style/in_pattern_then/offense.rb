case a
in b; c
    ^ Style/InPatternThen: Do not use `in b`. Use `in b then` instead.
end
case a
in b, c, d; e
          ^ Style/InPatternThen: Do not use `in b, c, d`. Use `in b, c, d then` instead.
end
case a
in 0 | 1 | 2; x
            ^ Style/InPatternThen: Do not use `in 0 | 1 | 2`. Use `in 0 | 1 | 2 then` instead.
end
