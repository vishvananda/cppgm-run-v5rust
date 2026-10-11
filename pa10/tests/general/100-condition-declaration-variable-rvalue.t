// HHC-110
int f(const unsigned char * p) {
  int n = 0;
  while (unsigned char c = *p++) {
    n += c;
  }
  return n;
}

int main() {
  return 0;
}
