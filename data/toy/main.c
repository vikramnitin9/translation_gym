#include <stdio.h>
#include "common.h"             // <-- brings in struct Point once
#include "math_functions.h"
#include "string_functions.h"

// Global state
static int global_counter = 0;
int test_global_extern = 1;     // definition, not "extern ... ="

// Reset the counter to zero, return its previous value
int reset_counter(void) {
    int prev = global_counter;
    global_counter = 0;
    return prev;
}

// Double the counter in place, return the new value
int double_counter(void) {
    global_counter *= 2;
    return global_counter;
}

int main(void) {
    // 1) reset (silently)
    (void)reset_counter();
    // 2) bump by 1
    global_counter++;
    // 3) double it
    (void)double_counter();

    // Now the original toy logic:
    struct Point p = {5, 3};
    struct Point q = {9, 1};
    struct Point mid = midpoint(p, q);
    (void)mid;  // force use

    printf("Sum: %d\n", add( p.x, p.y ));
    printf("Difference: %d\n", subtract( p.x, p.y ));
    printf("Concatenation: %s\n", concatenate("Hello, ", "world!"));

    return 0;
}
