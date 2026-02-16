def foo
  @bar ||= compute
  ^^^^ Naming/MemoizedInstanceVariableName: Memoized variable `@bar` does not match method name `foo`.
end
def something
  @other ||= calculate
  ^^^^^^ Naming/MemoizedInstanceVariableName: Memoized variable `@other` does not match method name `something`.
end
def value
  @cached ||= fetch
  ^^^^^^^ Naming/MemoizedInstanceVariableName: Memoized variable `@cached` does not match method name `value`.
end
