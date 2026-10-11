template<class T>
struct promise;

template<class T>
struct promise<T&> {
  template<class Allocator>
  promise(int, const Allocator&);
};

template<class T>
template<class Alloc>
promise<T&>::promise(int, const Alloc&) {}

int main() {
  return 0;
}
