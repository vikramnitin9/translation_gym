CC = clang
CFLAGS = -Wall -Wextra -std=c11
SRCS = avl_bf.c avl_data.c avl_test.c
OBJS = $(SRCS:.c=.o)
TARGET = avl-tree-test

all: clean $(OBJS)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^

%.o: %.c
	$(CC) $(CFLAGS) -c $<

clean:
	rm -f $(OBJS) $(TARGET)

macros:
	@for file in $(SRCS); do \
		echo "Expanding macros in $$file..."; \
		$(CC) $(CFLAGS) -E -CC -P $$file -o $$file.tmp && mv $$file.tmp $$file; \
	done

.PHONY: all clean macros
