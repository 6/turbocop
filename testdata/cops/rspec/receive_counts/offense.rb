expect(foo).to receive(:bar).exactly(1).times
                             ^^^^^^^^^^^^^^^^^ RSpec/ReceiveCounts: Use `.once` instead of `.exactly(1).times`.

expect(foo).to receive(:bar).exactly(2).times
                             ^^^^^^^^^^^^^^^^^ RSpec/ReceiveCounts: Use `.twice` instead of `.exactly(2).times`.

expect(foo).to receive(:bar).at_least(1).times
                             ^^^^^^^^^^^^^^^^^^ RSpec/ReceiveCounts: Use `.at_least(:once)` instead of `.at_least(1).times`.

expect(foo).to receive(:bar).at_most(2).times
                             ^^^^^^^^^^^^^^^^^ RSpec/ReceiveCounts: Use `.at_most(:twice)` instead of `.at_most(2).times`.
