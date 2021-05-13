# Indents are a holy sin for conditionals; only use them for logical understanding while modifying the
# makefile. Indent back to origin once you're done modifying

# first assign constants; **don't** modify them throughout
CONST_WIN_RUST_FLAGS := -Ctarget-feature=+crt-static
CONST_CARGO_BUILD_VERBOSE := cargo build --verbose
CONST_CARGO_BUILD_VERBOSE_RELEASE := $(CONST_CARGO_BUILD_VERBOSE) --release
CONST_CARGO_RUN := cargo run
CONST_PROJECT_SKYD_NOART := -p skyd -- --noart --nosave
CONST_CARGO_TEST := cargo test
CONST_CARGO_TEST_SINGLE_THREAD := -- --test-threads=1
CONST_WINDOWS_START_BACKGROUND := START /B
CONST_UNIX_START_BACKGROUND := &

# now generate the target specific build commands
CARGO_BUILD_DEBUG_FULL := $(CONST_CARGO_BUILD_VERBOSE)
CARGO_BUILD_RELEASE_FULL := $(CONST_CARGO_BUILD_VERBOSE_RELEASE)
CARGO_RUN_DEBUG_SKYD := $(CONST_CARGO_RUN)
CARGO_TEST := $(CONST_CARGO_TEST)
CLEAN_DIR :=
KILL_PROCESS :=
ifneq ($(origin TARGET),undefined)
# a target is defined
CARGO_BUILD_DEBUG_FULL += --target ${TARGET}
CARGO_BUILD_RELEASE_FULL += --target ${TARGET}
CARGO_RUN_DEBUG_SKYD += --target ${TARGET}
CARGO_TEST += --target ${TARGET}
endif
CARGO_RUN_DEBUG_SKYD += $(CONST_PROJECT_SKYD_NOART)
CARGO_TEST += $(CONST_CARGO_TEST_SINGLE_THREAD)
ifeq ($(OS),Windows_NT)
# windows host OS; add the windows rustflags
CARGO_BUILD_DEBUG_FULL = $(WIN_RUST_FLAGS) + $(CARGO_BUILD_DEBUG_FULL)
CARGO_BUILD_RELEASE_FULL = $(WIN_RUST_FLAGS) + $(CARGO_BUILD_RELEASE_FULL)
# also add the `START /B` to the run
CARGO_RUN_DEBUG_SKYD = $(CONST_WINDOWS_START_BACKGROUND) + $(CARGO_RUN_DEBUG_SKYD)
# and add the rd command
CLEAN_DIR += rmdir target data /s /q
# and add the kill
KILL_PROCESS += taskkill /f /t /im skyd.exe
else
# host is not Windows; just add the background task flag for *nix
CARGO_RUN_DEBUG_SKYD += $(CONST_UNIX_START_BACKGROUND)
# add the rm command
CLEAN_DIR += rm -rf target data
# and add the kill
KILL_PROCESS := pkill skyd
endif

debug:
	$(CARGO_BUILD_DEBUG_FULL)

release:
	$(CARGO_BUILD_RELEASE_FULL)
test:
	$(CARGO_RUN_DEBUG_SKYD)
	$(CARGO_TEST)
	$(KILL_PROCESS)
clean:
	$(CLEAN_DIR)
