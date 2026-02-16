folders = %x(find . -type d).split
          ^^^^^^^^^^^^^^^^^^^ Style/CommandLiteral: Use backticks around command string.

result = %x(ls -la)
         ^^^^^^^^^^ Style/CommandLiteral: Use backticks around command string.

output = %x(echo hello)
         ^^^^^^^^^^^^^^ Style/CommandLiteral: Use backticks around command string.
