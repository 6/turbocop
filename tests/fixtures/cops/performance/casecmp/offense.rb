str.downcase == "other"
^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase ==`.
str.upcase == "OTHER"
^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `upcase ==`.
str.downcase == 'string'
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `downcase ==`.
str.upcase == 'string'
^^^^^^^^^^^^^^^^^^^^^^ Performance/Casecmp: Use `casecmp` instead of `upcase ==`.
