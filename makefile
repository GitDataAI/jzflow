OUTPUT = dist

$(OUTPUT):
	mkdir -p $(OUTPUT)

build-cd: $(OUTPUT)
	cargo build -p compute_unit_runner --release
	cp target/release/compute_unit_runner $(OUTPUT)/compute_unit_runner

build-dp: $(OUTPUT)
	cargo build -p dp_runner --release
	cp target/release/dp_runner $(OUTPUT)/dp_runner

build: build-cd build-dp
	cargo build --release

docker_cd: build-cd
	docker build -f ./script/cd.dockerfile -t jz-action/compute_unit_runner:latest .

docker_dp: build-dp
	docker build -f ./script/dp.dockerfile -t jz-action/dp_runner:latest .

docker: docker_cd docker_dp

minikube-env:
	@echo "Setting up Docker environment for Minikube"
	@eval $(minikube -p minikube docker-env)

minikube-docker: minikube-env docker
	docker push jz-action/compute_unit_runner:latest
	docker push jz-action/dp_runner:latest

clean:
	rm -rf $(OUTPUT) target


docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
    -i /local/swagger.yml \
    -g rust \
	--skip-validate-spec \
    -o /local/dist