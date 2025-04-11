CFLAGS=-I./include -g
SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)

.PHONY: all
all: clean uniq

uniq: $(OBJS)
	gcc $^ -o $@

%.o: %.c
	gcc $(CFLAGS) -c $< -o $@

.PHONY: clean
clean:
	rm -rf $(OBJS) uniq
