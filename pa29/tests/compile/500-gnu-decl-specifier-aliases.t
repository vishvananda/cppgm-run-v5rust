typedef __signed char i8;
typedef unsigned long word;

extern __const int g;

int f(__volatile int* p) {
  return *p;
}

int main() {
  return 0;
}
