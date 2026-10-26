extern "C" unsigned int narrow_sum(unsigned long a, unsigned long b) {
    unsigned long value = a + 0xfffffff9UL;
    value += b;
    return static_cast<unsigned int>(value);
}

extern "C" unsigned long displaced_sum(unsigned long a, unsigned long b) {
    return (a + 7) + b;
}

extern "C" unsigned int narrow_load(volatile unsigned long* source) {
    unsigned long value = *source;
    return static_cast<unsigned int>(value);
}

extern "C" unsigned long narrow_twice(unsigned int a, unsigned int b) {
    unsigned long first = a;
    unsigned long second = b;
    return static_cast<unsigned int>(first + second);
}
