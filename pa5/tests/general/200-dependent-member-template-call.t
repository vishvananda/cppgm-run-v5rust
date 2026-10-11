template<class T>
struct box {
  template<class U>
  U get();

  template<class U>
  U call() {
    return this->template get<U>();
  }
};
