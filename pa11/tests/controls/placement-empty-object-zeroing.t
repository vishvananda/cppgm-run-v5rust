// expect exit 0
void* operator new(unsigned long, void* address) noexcept { return address; }
struct Empty {};
int main() {
  char storage;
  Empty* object = ::new (&storage) Empty{};
  return object == 0;
}
