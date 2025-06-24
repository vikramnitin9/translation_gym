CFLAGS=-I./include -g
SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

.PHONY: all
all: clean tail

tail: $(OBJS)
	gcc $^ -o $@

%.o: %.c
	gcc $(CFLAGS) -c $< -o $@

macros:
	@for file in $(SRCS); do \
		echo "Expanding macros in $$file..."; \
		gcc $(CFLAGS) -E -P $$file -o $$file.tmp && mv $$file.tmp $$file; \
	done

.PHONY: clean macros
clean:
	rm -rf $(OBJS) tail
