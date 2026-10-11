void* operator new(decltype(sizeof(0)), void*);

void allocate(void* storage) {
  new (int);
  new (char*)();
  new (storage) int;
}
