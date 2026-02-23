User.all.where(active: true)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected. Remove `all` from the chain.
User.all.order(:name)
^^^^^^^^^^^^^^^^^^^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected. Remove `all` from the chain.
User.all.first
^^^^^^^^^^^^^^ Rails/RedundantActiveRecordAllMethod: Redundant `all` detected. Remove `all` from the chain.