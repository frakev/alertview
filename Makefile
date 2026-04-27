IMAGE ?= ghcr.io/frakev/alertview:latest

# MicroK8s: import image directly into containerd (no registry push needed)
CTR   ?= /snap/microk8s/current/bin/ctr \
          --address /var/snap/microk8s/common/run/containerd.sock \
          --namespace k8s.io
KUBECTL ?= microk8s kubectl

.PHONY: build push deploy restart all

build:
	docker build -t $(IMAGE) .

# MicroK8s only: load image into local containerd
push: build
	docker save $(IMAGE) | $(CTR) images import -

deploy:
	$(KUBECTL) apply -f 01-namespace.yaml
	$(KUBECTL) apply -f 02-configmap.yaml
	$(KUBECTL) apply -f 03-deployment.yaml
	$(KUBECTL) apply -f 04-service.yaml
	$(KUBECTL) apply -f 05-ingress.yaml

restart:
	$(KUBECTL) rollout restart deployment/alertview -n alertview

# Build + load into containerd + rolling restart
all: push restart
