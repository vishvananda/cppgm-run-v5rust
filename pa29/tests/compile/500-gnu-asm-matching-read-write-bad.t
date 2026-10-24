int f(int x) {
  __asm__("addl %1, %0" : "+r"(x) : "0"(x));
  return x;
}
