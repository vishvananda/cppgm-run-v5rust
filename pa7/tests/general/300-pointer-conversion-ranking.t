int sink(bool) { return 1; }
int sink(const void*) { return 2; }

int main() {
  int x = 0;
  return sink(&x);
}
