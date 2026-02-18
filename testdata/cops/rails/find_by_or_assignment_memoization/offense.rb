@current_user ||= User.find_by(id: session[:user_id])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.

@post ||= Post.find_by(slug: params[:slug])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.

@team ||= Team.find_by(name: "default")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.
