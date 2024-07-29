OUTPUT = dist

$(OUTPUT):
	mkdir -p $(OUTPUT)

################### build crates
build-cd: $(OUTPUT)
	cargo build -p compute_unit_runner --release --bin compute_unit_runner
	cp target/release/compute_unit_runner $(OUTPUT)/compute_unit_runner

build-dp: $(OUTPUT)
	cargo build -p dp_runner --release
	cp target/release/dp_runner $(OUTPUT)/dp_runner



build: build-cd build-dp
	cargo build --release

docker_cd: build-cd
	docker build -f ./crates/compute_unit_runner/dockerfile -t jz-action/compute_unit_runner:latest .

docker_dp: build-dp
	docker build -f ./crates/dp_runner/dockerfile -t jz-action/dp_runner:latest .

################## build nodes
build-nodes: $(OUTPUT)
	cargo build -p jz_reader --release
	cp target/release/jz_reader $(OUTPUT)/jz_reader

	cargo build -p jz_writer --release
	cp target/release/jz_writer $(OUTPUT)/jz_writer

docker_nodes: build-nodes
	docker build -f ./nodes/jz_reader/dockerfile -t jz-action/jz_reader:latest .
	docker build -f ./nodes/jz_writer/dockerfile -t jz-action/jz_writer:latest .

################## minikube
docker: docker_cd docker_dp docker_nodes

minikube-env:
	@echo "Setting up Docker environment for Minikube"
	@eval $(minikube -p minikube docker-env)

minikube-docker: minikube-env docker

clean:
	rm -rf $(OUTPUT) target