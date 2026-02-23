params.expect(user: [:name, :age])
params.require(:name)
params.permit(:name)
params.require(:target).permit
params[:name]
params.fetch(:name)
