x = {
      a: 1,
      ^^^ Layout/FirstHashElementIndentation: Use 2 (not 6) spaces for indentation of the first element.
  b: 2
}
y = {
    c: 3,
    ^^ Layout/FirstHashElementIndentation: Use 2 (not 4) spaces for indentation of the first element.
  d: 4
}
z = {
        e: 5,
        ^^^ Layout/FirstHashElementIndentation: Use 2 (not 8) spaces for indentation of the first element.
  f: 6
}

buffer << {
  }
  ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the start of the line where the left brace is.

value = {
  a: 1
    }
    ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the start of the line where the left brace is.

wrap({
       a: 1
    })
    ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the first position after the preceding left parenthesis.

func(x: {
       a: 1,
       b: 2
   },
   ^ Layout/FirstHashElementIndentation: Indent the right brace the same as the parent hash key.
     y: {
       c: 1,
       d: 2
     })
