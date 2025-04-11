CFLAGS=-I./include -g
SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

.PHONY: all
all: clean cat lib.a

cat: $(OBJS)
	gcc $^ -o $@

%.o: %.c
	gcc $(CFLAGS) -c $< -o $@

lib.a: $(OBJS)
	ar rcs $@ $^

.PHONY: clean
clean:
	rm -rf $(OBJS) cat
