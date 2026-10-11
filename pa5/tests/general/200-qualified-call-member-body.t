namespace ns {
template<class T> void destroy_at(T*);
template<class T> T* addressof(T&);
}

struct box {
  void clear(box& other) {
    ns::destroy_at(&other);
    if (ns::addressof(other) != this) {
    }
  }
};
