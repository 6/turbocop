begin
  something
rescue => StandardError
       ^^ Lint/ConstantOverwrittenInRescue: `StandardError` is overwritten by `rescue =>`.
end

begin
  something
rescue => MyError
       ^^ Lint/ConstantOverwrittenInRescue: `MyError` is overwritten by `rescue =>`.
end

begin
  something
rescue => RuntimeError
       ^^ Lint/ConstantOverwrittenInRescue: `RuntimeError` is overwritten by `rescue =>`.
end
