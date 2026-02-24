x = 1; y = 2
     ^ Style/Semicolon: Do not use semicolons to terminate expressions.

a = 1; b = 2; c = 3
     ^ Style/Semicolon: Do not use semicolons to terminate expressions.
            ^ Style/Semicolon: Do not use semicolons to terminate expressions.

foo; bar
   ^ Style/Semicolon: Do not use semicolons to terminate expressions.

def guard; log('guard'); !@fail_guard; end
         ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                       ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                                     ^ Style/Semicolon: Do not use semicolons to terminate expressions.

def foo(a) x(1); y(2); z(3); end
               ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                     ^ Style/Semicolon: Do not use semicolons to terminate expressions.
                           ^ Style/Semicolon: Do not use semicolons to terminate expressions.
