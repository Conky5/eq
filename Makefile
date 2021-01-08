PREFIX=/usr/local
_INSTDIR=$(DESTDIR)$(PREFIX)
BINDIR?=$(INSTDIR)/bin
MANDIR?=$(_INSTDIR)/share/man
SCDOC := $(shell command -v scdoc 2> /dev/null)
.DEFAULT_GOAL := build

doc/eq.1: doc/eq.scd

docs: doc/eq.1
ifdef SCDOC
	scdoc < doc/eq.scd > doc/eq.1
endif

clean:
	@rm -f doc/eq.1
	@cargo clean

build: docs
	cargo build --release
	strip target/release/eq

release: build
	mkdir -p releases
	case $(shell uname -s) in Linux) OS=$(shell uname -p)-linux-gnu ;; Darwin) OS=apple-darwin ;; *) echo 'Unhandled OS' && exit 1 ;; esac; \
	  VERSION="$(shell ./target/release/eq --version | awk '{ print $$2 }')"; \
	  cd releases; \
	  DIRECTORY="eq-v$$VERSION-$(shell uname -m)-$$OS"; \
	  rm -rf $$DIRECTORY; \
	  mkdir $$DIRECTORY; \
	  cp ../target/release/eq $$DIRECTORY; \
	  cp ../doc/eq.1 $$DIRECTORY; \
	  tar czf $$DIRECTORY.tar.gz $$DIRECTORY

install:
	mkdir -p $(BINDIR)
	install -m 755 target/release/eq $(BINDIR)/eq
ifdef SCDOC
	mkdir -p $(MANDIR)/man1
	install -m644 doc/eq.1 $(MANDIR)/man1/eq.1
endif

uninstall:
	rm -f $(BINDIR)/eq
	rm -f $(MANDIR)/man1/eq.1

.PHONY: clean install build release
