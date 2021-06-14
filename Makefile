all: test

export SHELL:=bash

DROGUE_NS ?= drogue-iot

CLUSTER=minikube

ifeq ($CLUSTER,"minikube")
DOMAIN=$(minikube ip).nip.io
else ifeq ($CLUSTER,"kind")
DOMAIN=$(kubectl get node kind-control-plane -o jsonpath='{.status.addresses[?(@.type == "InternalIP")].address}').nip.io
else
$(error Unknown cluster type: $CLUSTER)
endif

CONSOLE_URL ?= $(shell minikube service -n "$(DROGUE_NS)" console --url)
API_URL ?= $(shell minikube service -n "$(DROGUE_NS)" console-backend --url)
RUST_LOG ?= info

.PHONY: start
start:
	-drg context delete system-tests
	-pkill geckodriver
	geckodriver &

.PHONY: stop
stop:
	-pkill geckodriver

.PHONY: test
test: start
	 trap "$(MAKE) stop" EXIT; $(MAKE) test-run

.PHONY: test-run
test-run:
	env \
		CONSOLE_URL=$(CONSOLE_URL) \
		API_URL=$(API_URL) \
		RUST_LOG=$(RUST_LOG) \
		TEST_USER=admin \
 		TEST_PASSWORD=admin123456 \
 		TEST_CLIENT_ID=test \
 		TEST_CLIENT_SECRET=$(shell kubectl -n "$(DROGUE_NS)" get secret keycloak-client-secret-test -o "jsonpath={.data.CLIENT_SECRET}" | base64 -d) \
		cargo test -- --test-threads=1 $(TESTS)
