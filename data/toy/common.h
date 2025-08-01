// common.h
#ifndef COMMON_H
#define COMMON_H

enum Color {
    RED,
    GREEN,
    BLUE
};

typedef enum Color Color;

struct Point {
    int x;
    int y;
    Color color;
};


#endif // COMMON_H