x = 'this text is too' \
    ' long'
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

y = 'this text contains a lot of' \
    '               spaces'
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

z = "another example" \
    " with leading space"
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

error = "go: example.com/tool@v1.0.0 requires\n" \
    "	github.com/example/dependency@v0.0.0-00010101000000-000000000000: invalid version"
     ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.

mixed = "foo #{bar}" \
  ' long'
   ^ Layout/LineContinuationLeadingSpace: Move leading spaces to the end of the previous line.
