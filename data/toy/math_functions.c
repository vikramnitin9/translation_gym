#include "math_functions.h"
#include "common.h"


int test = 0;

int add(int a, int b) {
    return a + b;
}

int subtract(int a, int b) {
    return a - b;
}


struct Point midpoint(struct Point a, struct Point b) {
    struct Point m;
    m.x = (a.x + b.x) / 2;
    m.y = (a.y + b.y) / 2;
    return m;
}