namespace n {
template<class T>
struct alloc {
  typedef int type;
};

template<class A, class B, class C>
struct pointer {};

template<class T>
struct box {
  T* ptr;

  template<class Y, int = 0>
  __attribute__((visibility("hidden"))) explicit box(Y* p) : ptr(p) {
    typedef typename alloc<Y>::type alloc_t;
    typedef pointer<Y*, forward_delete<T, Y>, alloc_t> block_t;
  }
};

template<class A, class B>
struct forward_delete {};
}

int main() { return 0; }
