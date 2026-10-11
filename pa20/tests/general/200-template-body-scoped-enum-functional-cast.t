typedef unsigned long size_t;

enum class element_count : size_t {};

int consume(element_count);

template<class T>
struct Alloc {
  int allocate(size_t n) {
    return consume(element_count(n));
  }
};

int main() { return 0; }
