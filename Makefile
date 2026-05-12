CC = cc
PREFIX ?= /usr/local
CFLAGS = -Wall -Wextra -Wpedantic -Werror -O2 -std=c11 -Ivendor/bearssl/inc
LDFLAGS =

UNAME_S := $(shell uname -s)
ifneq ($(UNAME_S),Darwin)
CFLAGS += -D_POSIX_C_SOURCE=200809L
endif

ifdef DEBUG
CFLAGS += -g -fsanitize=address -fno-omit-frame-pointer
LDFLAGS += -fsanitize=address
endif

SRCS = src/main.c src/drand.c src/https.c src/select.c src/timeutil.c src/format.c
TARGET = alea

BEARSSL_SRCS := $(shell find vendor/bearssl/src -name '*.c')
BEARSSL_OBJS := $(BEARSSL_SRCS:vendor/bearssl/src/%.c=vendor/bearssl/obj/%.o)
BEARSSL_CFLAGS = -O2 -std=c11 -Ivendor/bearssl/inc -Ivendor/bearssl/src
BEARSSL_LIB = vendor/bearssl/libbearssl.a

all: $(TARGET)

vendor/bearssl/obj/%.o: vendor/bearssl/src/%.c
	@mkdir -p $(dir $@)
	$(CC) $(BEARSSL_CFLAGS) -c $< -o $@

$(BEARSSL_LIB): $(BEARSSL_OBJS)
	ar rcs $@ $^

$(TARGET): $(SRCS) $(BEARSSL_LIB)
	$(CC) $(CFLAGS) -o $@ $(SRCS) $(BEARSSL_LIB) $(LDFLAGS)

# Cosmopolitan APE multi-platform binary (Linux/macOS/Windows x86-64+arm64)
COSMOCC ?= cosmocc
cosmo: $(SRCS) $(BEARSSL_SRCS)
	@mkdir -p vendor/bearssl/cosmo-obj
	@for f in $(BEARSSL_SRCS); do \
		out="vendor/bearssl/cosmo-obj/$$(basename $$f .c).o"; \
		$(COSMOCC) $(BEARSSL_CFLAGS) -c "$$f" -o "$$out" 2>/dev/null || exit 1; \
	done
	$(COSMOCC) -O2 -std=c11 -Ivendor/bearssl/inc -Wall -Wextra -Wpedantic \
		$(SRCS) vendor/bearssl/cosmo-obj/*.o -o alea.com

clean:
	rm -f $(TARGET) alea.com alea.com.dbg alea.aarch64.elf
	rm -rf vendor/bearssl/obj vendor/bearssl/cosmo-obj $(BEARSSL_LIB)

clean-app:
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

.PHONY: all clean clean-app debug cosmo
