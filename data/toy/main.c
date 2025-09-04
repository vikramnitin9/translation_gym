#include <stdio.h>
#include "common.h"             // <-- brings in struct Point once
#include "math_functions.h"
#include "string_functions.h"

// Global state
static int global_counter = 2;

// Reset the counter to zero, return its previous value
int reset_counter(void) {
    int prev = global_counter;
    global_counter = 0;
    return prev;
}



// Increment the counter in place, return the new value
int increment_counter(void) {
    global_counter++;
}

void check_counter(void) {
    printf("Counter: %d\n", global_counter);
    // 1) reset (silently)
    (void)reset_counter();
    printf("Counter: %d\n", global_counter);
    // 2) bump by 1
    (void)increment_counter();
    printf("Counter: %d\n", global_counter);
}

int main(void) {
    // Check the counter
    check_counter();

    struct Point p = {5, 3, RED};
    struct Point q = {9, 1, GREEN};
    struct Point mid = midpoint(p, q);

    printf("Sum: %d\n", add( p.x, p.y ));
    printf("Difference: %d\n", subtract( p.x, p.y ));
    printf("Midpoint: (%d, %d)\n", mid.x, mid.y);
    printf("Concatenation: %s\n", concatenate("Hello, ", "world!"));

    return 0;
}
