typedef decltype(sizeof(0)) size_t;

template<class T>
struct shared_ptr {
  int* __ptr_;
  int* __cntrl_;
};

template<class T>
struct weak_ptr {
  weak_ptr() noexcept;

  template<class Y>
  weak_ptr(const shared_ptr<Y>&) noexcept;

  int* __ptr_;
  int* __cntrl_;

  template<class U>
  int f(U);
};

template<class T>
inline weak_ptr<T>::weak_ptr() noexcept : __ptr_(nullptr), __cntrl_(nullptr) {}

template<class T>
template<class Y>
inline weak_ptr<T>::weak_ptr(const shared_ptr<Y>& __r) noexcept : __ptr_(__r.__ptr_), __cntrl_(__r.__cntrl_) {}

template<class T>
template<class U>
int weak_ptr<T>::f(U) {
  return 0;
}

template<class E, class U>
struct X {
  X(E&, size_t);
};

template<class E, class U>
X<E, U>::X(E&, size_t) {}

int main() {
  return 0;
}
