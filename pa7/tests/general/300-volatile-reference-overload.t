int f(volatile int& x) { return 1; }
int f(const int& x) { return 2; }

int main() {
  volatile int x = 0;
  return f(x);
}
