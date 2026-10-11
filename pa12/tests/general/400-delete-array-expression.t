int main() {
  int* p = (int*)::operator new[](sizeof(int) * 4);
  delete[] p;
  return p == 0;
}
