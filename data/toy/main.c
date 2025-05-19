#include <stdio.h>
#include "math_functions.h"
#include "string_functions.h"

// Global variable
int global_counter = 0;

// Struct definition
struct Point {
    int x;
    int y;
};

int main() {
    // Bump the global so the visitor still sees it
    global_counter++;

    // Use the struct locally so the visitor sees its declaration
    struct Point p;
    p.x = 5;
    p.y = 3;

    // Now call into your library functions
    printf("Sum: %d\n", add(p.x, p.y));
    printf("Difference: %d\n", subtract(p.x, p.y));
    printf("Concatenation: %s\n", concatenate("Hello, ", "world!"));

    return 0;
}
