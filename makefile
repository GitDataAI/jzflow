OUTPUT = dist

$(OUTPUT):
	mkdir -p $(OUTPUT)

################### build crates
build-jz: $(OUTPUT)
	cargo build --release --bin jz-flow
	cp target/release/jz-flow $(OUTPUT)/jz-flow

build-cd: $(OUTPUT)
	cargo build -p compute_unit_runner --release --bin compute_unit_runner
	cp target/release/compute_unit_runner $(OUTPUT)/compute_unit_runner

build: build-cd 
	cargo build --release

docker_cd: build-cd
	docker build -f ./crates/compute_unit_runner/dockerfile -t gitdatateam/compute_unit_runner:latest .

################## build nodes

build-nodes: $(OUTPUT)
	cargo build -p jz_reader --release
	cp target/release/jz_reader $(OUTPUT)/jz_reader

	cargo build -p jz_writer --release
	cp target/release/jz_writer $(OUTPUT)/jz_writer

	cargo build -p make_article --release
	cp target/release/make_article $(OUTPUT)/make_article

	cargo build -p list_files --release
	cp target/release/list_files $(OUTPUT)/list_files

	cargo build -p copy_in_place --release
	cp target/release/copy_in_place $(OUTPUT)/copy_in_place

docker_nodes: build-nodes
	docker build -f ./nodes/jz_reader/dockerfile -t gitdatateam/jz_reader:latest .
	docker build -f ./nodes/jz_writer/dockerfile -t gitdatateam/jz_writer:latest .
	docker build -f ./nodes/make_article/dockerfile -t gitdatateam/make_article:latest .
	docker build -f ./nodes/list_files/dockerfile -t gitdatateam/list_files:latest .
	docker build -f ./nodes/copy_in_place/dockerfile -t gitdatateam/copy_in_place:latest .

docker_examples:
	cd ./script/housing-prices && docker build -t gitdatateam/housing-prices:latest .

################## minikube
docker: docker_cd docker_nodes

docker-push: docker_nodes
	docker push gitdatateam/jz_reader:latest
	docker push gitdatateam/jz_writer:latest
	docker push gitdatateam/make_article:latest
	docker push gitdatateam/list_files:latest
	docker push gitdatateam/copy_in_place:latest

minikube-env:
	@echo "Setting up Docker environment for Minikube"
	@eval $(minikube -p minikube docker-env)

minikube-docker: minikube-env docker

clean:
	rm -rf $(OUTPUT) target