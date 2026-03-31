x = 1 +
        2
        ^^^ Layout/MultilineOperationIndentation: Align the operands of an expression in an assignment spanning multiple lines.
z = 5 +
      6
      ^^^ Layout/MultilineOperationIndentation: Align the operands of an expression in an assignment spanning multiple lines.
w = a &&
         b
         ^^^^ Layout/MultilineOperationIndentation: Align the operands of an expression in an assignment spanning multiple lines.

def skip?(tp)
  tp.path == __FILE__ ||
  tp.path == "<internal:trace_point>" ||
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineOperationIndentation: Use 2 (not 0) spaces for indenting an expression spanning multiple lines.
  @fiber != Fiber.current
  ^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineOperationIndentation: Use 2 (not 0) spaces for indenting an expression spanning multiple lines.
end

if children.length == 2 &&
  children.all? { |c| c.is_a?(RbiGenerator::Method) } &&
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineOperationIndentation: Align the operands of a condition in an `if` statement spanning multiple lines.
  children.count { |c| T.cast(c, RbiGenerator::Method).class_method } == 1
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineOperationIndentation: Align the operands of a condition in an `if` statement spanning multiple lines.
end

result = line_prefix + '├' + text_prefix +
  (colour ? Rainbow(message).green.bright.bold : message)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineOperationIndentation: Align the operands of an expression in an assignment spanning multiple lines.

file_table_entries.each do |file_table_entry|
  next if file_table_entry['sigil'] == 'Ignore' ||
  file_table_entry['strict'] == 'Ignore'
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Layout/MultilineOperationIndentation: Align the operands of a condition in an `if` statement spanning multiple lines.
end
