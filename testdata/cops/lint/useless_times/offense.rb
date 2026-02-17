0.times { something }
^^^^^^^^^^^^^^^^^^^^^ Lint/UselessTimes: Useless call to `0.times` detected.
1.times { something }
^^^^^^^^^^^^^^^^^^^^^ Lint/UselessTimes: Useless call to `1.times` detected.
-1.times { something }
^^^^^^^^^^^^^^^^^^^^^^ Lint/UselessTimes: Useless call to `-1.times` detected.
1.times
^^^^^^^ Lint/UselessTimes: Useless call to `1.times` detected.
