begin
  do_something

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
rescue

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected after the `rescue`.
  handle_error
end

begin
  something

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `ensure`.
ensure

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected after the `ensure`.
  cleanup
end
