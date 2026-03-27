a > b ? a : b
^^^^^^^^^^^^^ Style/MinMaxComparison: Use `[a, b].max` instead.

a < b ? a : b
^^^^^^^^^^^^^ Style/MinMaxComparison: Use `[a, b].min` instead.

a >= b ? b : a
^^^^^^^^^^^^^^ Style/MinMaxComparison: Use `[a, b].min` instead.

a <= b ? b : a
^^^^^^^^^^^^^^ Style/MinMaxComparison: Use `[a, b].max` instead.

(a >= b) ? b : a
^^^^^^^^^^^^^^^^ Style/MinMaxComparison: Use `[a, b].min` instead.

if userThreshold.nil?
  BlobConstants::DEFAULT_SINGLE_BLOB_PUT_THRESHOLD_IN_BYTES
elsif userThreshold <= 0
  raise ArgumentError, "Single Upload Threshold should be positive number"
elsif userThreshold < BlobConstants::MAX_SINGLE_UPLOAD_BLOB_SIZE_IN_BYTES
^ Style/MinMaxComparison: Use `[userThreshold, BlobConstants::MAX_SINGLE_UPLOAD_BLOB_SIZE_IN_BYTES].min` instead.
  userThreshold
else
  BlobConstants::MAX_SINGLE_UPLOAD_BLOB_SIZE_IN_BYTES
end

if MIN_SEGMENT_SIZE > segment_size
  MIN_SEGMENT_SIZE
elsif MAX_SEGMENT_SIZE < segment_size
^ Style/MinMaxComparison: Use `[MAX_SEGMENT_SIZE, segment_size].min` instead.
  MAX_SEGMENT_SIZE
else
  segment_size
end

def get_segment_size_for_split(segment_size)
  if MIN_SEGMENT_SIZE > segment_size
    MIN_SEGMENT_SIZE
  elsif MAX_SEGMENT_SIZE < segment_size
  ^ Style/MinMaxComparison: Use `[MAX_SEGMENT_SIZE, segment_size].min` instead.
    MAX_SEGMENT_SIZE
  else
    segment_size
  end
end

if run_time > Delayed::Worker.max_run_time
^ Style/MinMaxComparison: Use `[run_time, Delayed::Worker.max_run_time].min` instead.
  Delayed::Worker.max_run_time
else
  run_time
end

temp = (omax < nmax) ? omax : nmax
       ^ Style/MinMaxComparison: Use `[omax, nmax].min` instead.

if x > @rl_end
  @rl_end
elsif (x < 0)
^ Style/MinMaxComparison: Use `[x, 0].max` instead.
  0
else
  x
end

if @a < @b then @a else @b end
^ Style/MinMaxComparison: Use `[@a, @b].min` instead.

if physical_balance >= 1000
  physical_balance - 1000
elsif physical_balance <= 0
^ Style/MinMaxComparison: Use `[physical_balance, 0].max` instead.
  0
else
  physical_balance
end
