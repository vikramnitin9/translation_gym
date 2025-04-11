CFLAGS=-I./include -g
SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)
LDFLAGS= -lcrypto -lssl -lgmp

.PHONY: all
all: clean getlimits

getlimits: $(OBJS)
	gcc $^ $(LDFLAGS) -o $@

%.o: %.c
	gcc $(CFLAGS) -c $< -o $@

.PHONY: clean
clean:
	rm -rf $(OBJS) getlimits
