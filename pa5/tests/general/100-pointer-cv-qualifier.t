// HHC-007
int f(char * const *p);
volatile char * const volatile * const q = nullptr;

int main() {
  return 0;
}
