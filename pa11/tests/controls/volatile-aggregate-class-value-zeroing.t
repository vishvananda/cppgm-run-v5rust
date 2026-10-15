// expect exit 0
void* operator new(unsigned long, void* address) noexcept { return address; }
struct Member { int n = 3; };
struct Object { Member member; };
int main() {
  alignas(16) char storage[128];
  volatile Object* object = ::new (storage) volatile Object{};
  return object->member.n != 3;
}
