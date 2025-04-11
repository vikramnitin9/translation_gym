CC = gcc
CFLAGS = -Wall -Wextra -std=c11
SRCS = main.c math_functions.c string_functions.c
OBJS = $(SRCS:.c=.o)
TARGET = toy

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^

%.o: %.c
	$(CC) $(CFLAGS) -c $<

clean:
	rm -f $(OBJS) $(TARGET)

.PHONY: all clean
