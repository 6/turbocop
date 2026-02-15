head :ok
head :not_found
head :no_content
head :created
head :internal_server_error
head :unprocessable_content
head :content_too_large
head 200
head 404
render json: {}, status: :ok
