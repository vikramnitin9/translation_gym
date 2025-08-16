CC = clang
CFLAGS = -Wall -Wextra -std=c11
SRCS = zopfli.c
OBJS = $(SRCS:.c=.o)
TARGET = zopfli-test

override CFLAGS := -W -Wall -Wextra -ansi -lm -O3 -Wno-unused-function -Wno-pointer-sign -fPIC $(CFLAGS)

all: $(OBJS)

$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) -o $@ $^ -lm

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
