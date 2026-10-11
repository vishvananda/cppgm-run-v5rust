__complex__ long double make_complex(long double re, long double im) {
  return __builtin_complex(re, im);
}

int main() {
  return 0;
}
