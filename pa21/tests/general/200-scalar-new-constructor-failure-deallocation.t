// Failed construction destroys the argument temporary and releases storage.
long long storage[8];
int allocations;
int cleanup_order;
int arguments_alive;

void* operator new(unsigned long) {
  ++allocations;
  return storage;
}
void operator delete(void*) noexcept {
  --allocations;
  cleanup_order = cleanup_order * 10 + 1;
}

struct argument {
  argument() { ++arguments_alive; }
  ~argument() noexcept { --arguments_alive; cleanup_order = cleanup_order * 10 + 2; }
};
struct item {
  item(const argument&) { throw 7; }
};

struct nullable_item {
  static void* operator new(unsigned long) noexcept { return nullptr; }
  static void operator delete(void*) noexcept { cleanup_order = 99; }
  nullable_item(const argument&) { throw 8; }
};

int main() {
  try { new item{argument()}; }
  catch (int value) {
    if (value != 7 || allocations != 0 || arguments_alive != 0 || cleanup_order != 21) return 1;
    // C++11 permits evaluating arguments before null allocation; destroy only
    // arguments that were constructed and never call the item constructor.
    nullable_item* skipped = new nullable_item{argument()};
    return skipped != nullptr || allocations != 0 || arguments_alive != 0 ||
      (cleanup_order != 21 && cleanup_order != 212);
  }
  return 2;
}
