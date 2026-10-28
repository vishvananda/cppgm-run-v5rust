int calls;
int next() { return ++calls; }
int selected() {
  static union { int value = next(); };
  return value;
}
int main() { return selected() == 1 && selected() == 1 && calls == 1 ? 0 : 1; }
