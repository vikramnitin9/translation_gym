#include <stdio.h>
#include "math_functions.h"
#include "string_functions.h"

// Global variable
int global_counter = 0;


int main() {
    // Bump the global so the visitor still sees it
    global_counter++;

     struct Point p;
     p.x = 5;
     p.y = 3;


    // Now call into your library functions
    struct Point q = { 9, 1 };
    struct Point mid = midpoint(p, q);
    (void)mid;  // use mid so it isn’t optimized away
    printf("Sum: %d\n", add(p.x, p.y));
    printf("Difference: %d\n", subtract(p.x, p.y));
    printf("Concatenation: %s\n", concatenate("Hello, ", "world!"));

    return 0;
}
