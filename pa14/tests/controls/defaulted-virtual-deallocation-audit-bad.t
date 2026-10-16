// expect reject
// Defining a demanded out-of-class defaulted virtual destructor selects its
// deallocation function; a deleted selection rejects before LowIR emission.
template<class T> struct Owner {
  virtual ~Owner();
  static void operator delete(void*) = delete;
};
template<class U> Owner<U>::~Owner() = default;
int main() {
  Owner<int> object;
  return 0;
}
