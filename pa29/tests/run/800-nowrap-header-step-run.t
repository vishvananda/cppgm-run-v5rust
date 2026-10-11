// N3485 5.2.6p1, 4.7p2-p3: a narrow loop counter stepped in the loop's
// condition (`while (i++ < n)`) is stepped once more on the pass that leaves
// the loop: with n its type's maximum the unsigned char wraps to 0 and the
// signed char becomes -128 (the conversion GCC and Clang define)
