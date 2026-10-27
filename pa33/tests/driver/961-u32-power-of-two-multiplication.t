// Preserve the frozen 32-bit multiply encoding policy and its zero extension.
__attribute__((noinline)) unsigned twice(unsigned value) { return value * 2u; }
__attribute__((noinline)) unsigned eight(unsigned value) { return value * 8u; }
__attribute__((noinline)) unsigned large(unsigned value) { return value * 1024u; }
__attribute__((noinline)) unsigned high(unsigned value) { return value * 0x80000000u; }
__attribute__((noinline)) unsigned odd(unsigned value) { return value * 3u; }
int main() {
  volatile unsigned inputs[] = {0u, 1u, 0x7fffffffu, 0x80000000u, 0xffffffffu};
  for (int i = 0; i < 5; ++i) {
    unsigned value = inputs[i];
    if (twice(value) != (value << 1)) return 1;
    if (eight(value) != (value << 3)) return 2;
    if (large(value) != (value << 10)) return 3;
    if (high(value) != (value << 31)) return 4;
    if (odd(value) != value + value + value) return 5;
  }
  return 0;
}
