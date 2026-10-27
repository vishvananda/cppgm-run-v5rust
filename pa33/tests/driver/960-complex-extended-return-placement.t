// Optimized placement must see both words of an extended complex return as
// one terminating MIR operation, including early and cold return paths.
typedef __complex__ long double Complex;
volatile int calls;
__attribute__((noinline)) void touch() { ++calls; }
__attribute__((noinline, noreturn)) void stop() { for (;;) {} }
__attribute__((noinline)) Complex early(int n, Complex value) {
  if (n == 0) return value;
  touch();
  if (n > 0) return value;
  __real__ value += 1;
  return value;
}
__attribute__((noinline)) Complex cold(int n, Complex value) {
  if (n == 0) stop();
  if (n > 1) {
    __real__ value += 1;
    return value;
  }
  __imag__ value += 2;
  return value;
}
int main() {
  Complex input;
  __real__ input = 4;
  __imag__ input = 6;
  for (int n = -1; n <= 1; ++n) {
    Complex result = early(n, input);
    if (__real__ result != (n < 0 ? 5 : 4) || __imag__ result != 6) return 1;
  }
  Complex first = cold(1, input);
  Complex second = cold(2, input);
  if (__real__ first != 4 || __imag__ first != 8) return 2;
  if (__real__ second != 5 || __imag__ second != 6) return 3;
  return calls - 2;
}
