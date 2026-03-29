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

begin
  recover
rescue=>e

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected after the `rescue`.
  handle_error
end

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.

^ Layout/EmptyLinesAroundExceptionHandlingKeywords: Extra empty line detected before the `rescue`.
