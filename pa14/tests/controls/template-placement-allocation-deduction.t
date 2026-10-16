// expect exit 0
int storage;
template<class T> void* operator new(unsigned long, T* destination) {
  return destination;
}
int main() {
  int* value = new(&storage) int(3);
  return *value - 3;
}
