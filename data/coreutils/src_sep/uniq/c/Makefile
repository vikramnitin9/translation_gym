CFLAGS=-I./include -g
SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

.PHONY: all
all: clean uniq

uniq: $(OBJS)
	clang $^ -o $@

%.o: %.c
	clang $(CFLAGS) -c $< -o $@

macros:
	@for file in $(SRCS); do \
		echo "Expanding macros in $$file..."; \
		clang $(CFLAGS) -E -CC -P $$file -o $$file.tmp && mv $$file.tmp $$file; \
	done

.PHONY: clean macros
clean:
	rm -rf $(OBJS) uniq
