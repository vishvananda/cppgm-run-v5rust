unsigned long count() {
  return 4;
}

int main() {
  bool* p = new bool[count()]();
  delete[] p;
  return 0;
}
