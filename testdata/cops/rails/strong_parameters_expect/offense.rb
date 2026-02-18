params.require(:user).permit(:name, :age)
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/StrongParametersExpect: Use `expect(...)` instead.

params.require(:user).permit(:name, some_ids: [])
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/StrongParametersExpect: Use `expect(...)` instead.

params.permit(user: [:name, :age]).require(:user)
       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/StrongParametersExpect: Use `expect(...)` instead.
