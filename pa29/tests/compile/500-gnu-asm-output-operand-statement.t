template<class T>
T f(T x) {
  __asm__("bswap %0" : "+r"(x));
  return x;
}

int main() {
  return 0;
}
