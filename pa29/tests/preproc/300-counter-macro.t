#ifndef __COUNTER__
#error missing __COUNTER__
#endif
#define JOIN_IMPL(a, b) a ## b
#define JOIN(a, b) JOIN_IMPL(a, b)
JOIN(value, __COUNTER__) JOIN(value, __COUNTER__)
