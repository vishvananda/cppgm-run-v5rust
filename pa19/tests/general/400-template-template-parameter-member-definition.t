template<class, class, class>
struct LayoutImpl;

template<class T, class Alloc, template<class, class, class> class Layout>
struct Buffer {
  typedef int size_type;
  void construct_at_end(size_type n);
};

template<class T, class Alloc, template<class, class, class> class Layout>
void Buffer<T, Alloc, Layout>::construct_at_end(size_type n) {}

int main() {
  return 0;
}
