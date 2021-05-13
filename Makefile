WIN_RUST_FLAGS := -Ctarget-feature=+crt-static
DBG_BUILD_VERBOSE := cargo build --verbose
NO_TARGET_SPECIFIED := No target specified. Defaulting to platform target
RUN_DBG := cargo run -p skyd
START_ARGS := --nosave --noart
DBG_TEST_VERBOSE := cargo test --verbose
TESTS_SINGLE_THREAD := --test-threads=1
RELASE_VERBOSE:= cargo build --release --verbose
debug-full:
ifeq ($(OS),Windows_NT)
	RUSTFLAGS=$(WIN_RUST_FLAGS) $(DBG_BUILD_VERBOSE)
else
	$(DBG_BUILD_VERBOSE)
endif

debug-server:
ifeq ($(OS),Windows_NT)
ifeq ($(origin TARGET),undefined)
	$(info $(NO_TARGET_SPECIFIED) for building skyd (debug, Windows))
	RUSTFLAGS=$(WIN_RUST_FLAGS) $(DBG_BUILD_VERBOSE) -p skyd
else
	$(info Building skyd for target ${TARGET})
	RUSTFLAGS=$(WIN_RUST_FLAGS) $(DBG_BUILD_VERBOSE) --target ${TARGET} -p skyd
endif
else
ifeq ($(origin TARGET),undefined)
	$(info $(NO_TARGET_SPECIFIED) for building skyd (debug))
	$(DBG_BUILD_VERBOSE) -p skyd
else
	$(info Building skyd for target ${TARGET})
	$(DBG_BUILD_VERBOSE) --target ${TARGET} -p skyd
endif
endif

test: debug-server
ifeq ($(OS),Windows_NT)
ifeq ($(origin TARGET),undefined)
	START /B $(RUN_DBG) -- $(START_ARGS)
	$(DBG_TEST_VERBOSE) -- $(TESTS_SINGLE_THREAD)
else
	START /B $(RUN_DBG) --target ${TARGET} --$(START_ARGS)
	$(DBG_TEST_VERBOSE) -- $(TESTS_SINGLE_THREAD)
endif
else
ifeq ($(origin TARGET),undefined)
	$(RUN_DBG) -- $(START_ARGS) &
	$(DBG_TEST_VERBOSE) -- $(TESTS_SINGLE_THREAD)
else
	$(RUN_DBG) --target ${TARGET} -- $(START_ARGS) &
	$(DBG_TEST_VERBOSE) --target ${TARGET} -- $(TESTS_SINGLE_THREAD)
endif
endif

clean:
ifeq ($(OS),Windows_NT)
	rd /s /q target
else
	rm -rf target
endif

release:
ifeq ($(OS),Windows_NT)
ifeq ($(origin TARGET),undefined)
	$(info $(NO_TARGET_SPECIFIED))
	RUSTFLAGS=$(WIN_RUST_FLAGS) $(RELASE_VERBOSE) -p skyd
else
	$(info Building for target ${TARGET})
	RUSTFLAGS=$(WIN_RUST_FLAGS) $(RELASE_VERBOSE) --target ${TARGET} -p skyd
endif
else
ifeq ($(origin TARGET),undefined)
	$(info $(NO_TARGET_SPECIFIED))
	$(RELASE_VERBOSE) -p skyd
else
	$(RELASE_VERBOSE) --target ${TARGET} -p skyd
endif
endif
