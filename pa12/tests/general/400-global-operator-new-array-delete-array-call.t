int main() {
  void* p = ::operator new[](sizeof(int) * 4);
  ::operator delete[](p);
  return p == 0;
}
