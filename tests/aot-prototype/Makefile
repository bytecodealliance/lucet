LLVM_PATH=/usr/local/opt/llvm
WABT_PATH=~/Code/wasm/wabt

CC=$(LLVM_PATH)/bin/clang
CXX=$(LLVM_PATH)/bin/clang++
LLVM_CONFIG=$(LLVM_PATH)/bin/llvm-config
LLVM_CFLAGS := $(shell $(LLVM_CONFIG) --cflags)
LLVM_LDFLAGS := $(shell $(LLVM_CONFIG) --cxxflags --ldflags --libs all --system-libs)

main.o: main.cc
	$(CXX) -g -std=c++11 $(LLVM_CFLAGS) -I $(WABT_PATH)/src/ -I $(WABT_PATH)/out/clang/Debug main.cc -c -o main.o

main: main.o
	$(CXX) -g main.o -o main -L $(WABT_PATH)/out/clang/Debug/ -l wabt $(LLVM_LDFLAGS)

check: main
	./main hello.wasm
	$(LLVM_PATH)/bin/llvm-dis -o output.ll output.bc
