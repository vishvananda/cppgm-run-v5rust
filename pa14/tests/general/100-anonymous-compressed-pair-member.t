template<class T, class D>
class unique_ptr {
  struct {
    T* __ptr_;
    D __deleter_;
  };
};

int main() {
  return 0;
}
