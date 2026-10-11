// HHC-104
int f(const char* p) {
  const char* q = p++;
  return q == p ? 0 : 1;
}

int main() {
  return 0;
}
