template<class T>
T f(T x) {
  asm("nop");
  return x;
}

int main() {
  return 0;
}
