CC = cc
PREFIX ?= /usr/local
CFLAGS = -Wall -Wextra -Wpedantic -Werror -O2 -std=c11
LDFLAGS = -lcurl

UNAME_S := $(shell uname -s)
ifneq ($(UNAME_S),Darwin)
CFLAGS += -D_POSIX_C_SOURCE=200809L $(shell pkg-config --cflags libcrypto)
LDFLAGS += $(shell pkg-config --libs libcrypto)
endif

# Enable AddressSanitizer for debug builds
# NOTE: ASan + libcurl deadlocks on macOS (SecureTransport/threaded DNS).
# Use debug builds only for offline/unit testing, not network calls.
ifdef DEBUG
CFLAGS += -g -fsanitize=address -fno-omit-frame-pointer
LDFLAGS += -fsanitize=address
endif

SRCS = src/main.c src/drand.c src/select.c src/timeutil.c src/format.c
TARGET = alea

all: $(TARGET)

$(TARGET): $(SRCS)
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

clean:
	rm -f $(TARGET)

install: $(TARGET)
	install -d $(DESTDIR)$(PREFIX)/bin
	install -m 755 $(TARGET) $(DESTDIR)$(PREFIX)/bin/
	install -d $(DESTDIR)$(PREFIX)/share/man/man1
	install -m 644 alea.1 $(DESTDIR)$(PREFIX)/share/man/man1/

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/$(TARGET)
	rm -f $(DESTDIR)$(PREFIX)/share/man/man1/alea.1

lint:
	clang-tidy --checks='bugprone-*,clang-analyzer-*,-clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling,-bugprone-multi-level-implicit-pointer-conversion' $(SRCS) -- $(CFLAGS)

debug:
	$(MAKE) DEBUG=1

.PHONY: all clean debug
