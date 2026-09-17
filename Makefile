# Run the generated binary with:
#   LD_LIBRARY_PATH=target/debug ./target/debug/hello

.PHONY: all cargo clean

all: cargo target/debug/hello

cargo:
	cargo build

target/debug/hello: demo/hello.c \
                    include/windows.h \
                    target/debug/include/win32_impl.h \
                    target/debug/libocside.a
	gcc demo/hello.c -I include -I target/debug/include -Ltarget/debug -locside -o $@

clean:
	cargo clean
	rm -f target/debug/hello
