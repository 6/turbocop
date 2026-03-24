@current_user ||= User.find_by(id: session[:user_id])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.

@post ||= Post.find_by(slug: params[:slug])
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.

@team ||= Team.find_by(name: "default")
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.

# ||= find_by nested inside an outer ||= begin...end block
@records ||= begin
  Class.new(base_record) do
    def item
      @item ||= records.items.find_by(id: item_id)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.
    end
  end
end

# def inside if block — the ||= find_by is the sole body of the def,
# so RuboCop's on_def fires (which doesn't check if ancestors)
if Rails.env.development?
  def gavin
    @gavin ||= hooli.users.find_by(email: "gavin@hooli.com")
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.
  end

  def hooli
    @hooli ||= Organization.find_by(name: "Hooli")
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/FindByOrAssignmentMemoization: Avoid memoizing `find_by` results with `||=`.
  end
end
