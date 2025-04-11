#include <stdio.h>
#include "math_functions.h"
#include "string_functions.h"

int main() {
    int a = 5, b = 3;
    printf("Sum: %d\n", add(a, b));
    printf("Difference: %d\n", subtract(a, b));
    printf("Concatenation: %s\n", concatenate("Hello, ", "world!"));
    return 0;
}