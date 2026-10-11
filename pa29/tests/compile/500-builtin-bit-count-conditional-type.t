int bit_width(unsigned long long value) {
  return sizeof(value) == sizeof(unsigned long long)
           ? __builtin_clzll(+value)
           : __builtin_clz(+value);
}

int main() {
  return bit_width(1) >= 0 ? 0 : 1;
}
