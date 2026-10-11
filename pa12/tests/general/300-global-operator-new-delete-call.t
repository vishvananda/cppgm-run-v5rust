int main() {
  void* p = ::operator new(sizeof(int));
  ::operator delete(p);
  return p == 0;
}
