# Indents are a holy sin for conditionals; only use them for logical understanding while modifying the
# makefile. Indent back to origin once you're done modifying

# first assign constants; **don't** modify them throughout
CONST_WIN_RUST_FLAGS := -Ctarget-feature=+crt-static
CONST_CARGO_BUILD_VERBOSE := cargo build --verbose
CONST_CARGO_BUILD_VERBOSE_RELEASE := $(CONST_CARGO_BUILD_VERBOSE) --release
CONST_CARGO_RUN := cargo run
CONST_PROJECT_SKYD := -p skyd
CONST_PROJECT_SKYD_NOART := $(CONST_PROJECT_SKYD) -- --noart --nosave
CONST_CARGO_TEST := cargo test
CONST_CARGO_TEST_SINGLE_THREAD := -- --test-threads=1
CONST_WINDOWS_START_BACKGROUND := START /B
CONST_UNIX_START_BACKGROUND := &

# now generate the target specific build commands
_CARGO_BUILD_DEBUG_FULL := $(CONST_CARGO_BUILD_VERBOSE)
_CARGO_BUILD_RELEASE_FULL := $(CONST_CARGO_BUILD_VERBOSE_RELEASE)
_CARGO_RUN_DEBUG_SKYD := $(CONST_CARGO_RUN)
_CARGO_BUILD_DEBUG_SKYD:= $(CONST_CARGO_BUILD_VERBOSE)
CARGO_TEST := $(CONST_CARGO_TEST)
CLEAN_DIR :=
KILL_PROCESS :=
CLEAN_DATA :=
ifneq ($(origin TARGET),undefined)
# a target is defined
_CARGO_BUILD_DEBUG_FULL += --target ${TARGET}
_CARGO_BUILD_RELEASE_FULL += --target ${TARGET}
_CARGO_RUN_DEBUG_SKYD += --target ${TARGET}
_CARGO_BUILD_DEBUG_SKYD += --target ${TARGET}
CARGO_TEST += --target ${TARGET}
endif
_CARGO_BUILD_DEBUG_SKYD += $(CONST_PROJECT_SKYD)
_CARGO_RUN_DEBUG_SKYD += $(CONST_PROJECT_SKYD_NOART)
CARGO_TEST += $(CONST_CARGO_TEST_SINGLE_THREAD)
# the final commands
CARGO_BUILD_RELEASE_FULL :=
CARGO_BUILD_DEBUG_FULL :=
CARGO_BUILD_DEBUG_SKYD :=
CARGO_RUN_DEBUG_SKYD :=
ifeq ($(OS),Windows_NT)
# windows host OS; add the windows rustflags
CARGO_BUILD_DEBUG_FULL += $(WIN_RUST_FLAGS) 
CARGO_BUILD_RELEASE_FULL += $(WIN_RUST_FLAGS)
CARGO_BUILD_DEBUG_SKYD += $(WIN_RUST_FLAGS)
# also add the `START /B` to the run
CARGO_RUN_DEBUG_SKYD += $(CONST_WINDOWS_START_BACKGROUND)
CARGO_RUN_DEBUG_SKYD += $(_CARGO_RUN_DEBUG_SKYD)
# and add the rd command
CLEAN_DIR += rmdir target /s /q
CLEAN_DATA += rmdir data /s /q
# and add the kill
KILL_PROCESS += taskkill /f /t /im skyd.exe
else
# host is not Windows; just add the background task flag for *nix
CARGO_BUILD_DEBUG_SKYD += $(_CARGO_BUILD_DEBUG_SKYD)
CARGO_RUN_DEBUG_SKYD += $(CONST_UNIX_START_BACKGROUND)
# add the rm command
CLEAN_DIR += rm -rf target
CLEAN_DATA += rm -rf data
# and add the kill
KILL_PROCESS := sudo pkill skyd
endif
# prepare the final commands
CARGO_BUILD_RELEASE_FULL += $(_CARGO_BUILD_RELEASE_FULL)
CARGO_BUILD_DEBUG_FULL += $(_CARGO_BUILD_DEBUG_FULL)
CARGO_BUILD_DEBUG_SKYD += $(_CARGO_BUILD_DEBUG_SKYD)
# do we need additional software?
INSTALL_PREREQ :=
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Linux)
INSTALL_PREREQ += sudo apt update && sudo apt-get install musl-tools gcc-multilib -y
endif

default: debug
	@echo Finished building debug version
prerequisites:
	$(INSTALL_PREREQ)
	@echo Done checking and/or installing prerequisites
debug-server: prerequisites
	$(CARGO_BUILD_DEBUG_SKYD)
	@echo ---------------- BUILT SKYD DEBUG BINARY ----------------
debug: prerequisites
	$(CARGO_BUILD_DEBUG_FULL)
release: prerequisites
	$(CARGO_BUILD_RELEASE_FULL)
test: debug-server
	$(CARGO_RUN_DEBUG_SKYD)
	@echo ---------------- STARTED SKYTABLE SERVER ----------------
	$(CARGO_TEST)
	@echo Cleaning up processes ...
	@$(KILL_PROCESS)
	@echo Cleaning up data files ...
	@$(CLEAN_DATA)
clean:
	$(CLEAN_DIR)
	$(CLEAN_DATA)
