RUSTC ?= rustc

BUILD = $(CURDIR)/build

LIB_SRC = $(wildcard ${CURDIR}/src/*.rs ${CURDIR}/src/**/*.rs)
LIB_MAIN = $(CURDIR)/src/lib.rs
LIB_NAME = ${BUILD}/$(shell ${RUSTC} --crate-file-name ${LIB_MAIN})
TEST_BIN = ${BUILD}/$(shell ${RUSTC} --test --crate-file-name ${LIB_MAIN})

BENCH_SRC = $(wildcard ${CURDIR}/bench/*.rs ${CURDIR}/bench/**/*.rs)
BENCH_MAIN = $(CURDIR)/bench/bin.rs
BENCH_BIN = ${BUILD}/$(shell ${RUSTC} --crate-file-name ${BENCH_MAIN})

CRITERION_DIR := $(CURDIR)/lib/criterion
CRITERION_SRC = $(wildcard ${CRITERION_DIR}/src/*.rs ${CRITERION_DIR}/src/**/*.rs)
CRITERION_MAIN = ${CRITERION_DIR}/src/lib.rs
CRITERION_LIB_NAME = ${BUILD}/$(shell ${RUSTC} --crate-file-name ${CRITERION_MAIN})

QUICKCHECK_DIR := $(CURDIR)/lib/quickcheck
QUICKCHECK_SRC = $(wildcard ${QUICKCHECK_DIR}/src/*.rs ${QUICKCHECK_DIR}/src/**/*.rs)
QUICKCHECK_MAIN = ${QUICKCHECK_DIR}/src/lib.rs
QUICKCHECK_LIB_NAME = ${BUILD}/$(shell ${RUSTC} --crate-file-name ${QUICKCHECK_MAIN})

all: $(LIB_NAME)

.PHONY: test
test: $(TEST_BIN)
	@RUST_LOG=quickcheck $(TEST_BIN)

.PHONY: bench
bench: $(BENCH_BIN)
	$(BENCH_BIN)

$(LIB_NAME): $(QUICKCHECK_LIB_NAME) $(LIB_SRC)
	@mkdir -p ${BUILD}
	$(RUSTC) -L $(BUILD) --out-dir ${BUILD} ${LIB_MAIN}

$(TEST_BIN): $(QUICKCHECK_LIB_NAME) $(LIB_SRC)
	$(RUSTC) -L $(BUILD) --test --out-dir ${BUILD} ${LIB_MAIN}

$(CRITERION_LIB_NAME): $(CRITERION_SRC)
	@mkdir -p ${BUILD}
	$(RUSTC) -O --out-dir ${BUILD} $(CRITERION_MAIN)

$(QUICKCHECK_LIB_NAME): $(QUICKCHECK_SRC)
	@mkdir -p ${BUILD}
	$(RUSTC) -O --out-dir ${BUILD} $(QUICKCHECK_MAIN)

$(BENCH_BIN): $(LIB_NAME) $(CRITERION_LIB_NAME) $(BENCH_SRC)
	$(RUSTC) -L $(BUILD) --out-dir ${BUILD} ${BENCH_MAIN}

.PHONY: update
update:
	git submodule init
	git submodule update

.PHONY: clean
clean:
	@rm -rf ${BUILD}
