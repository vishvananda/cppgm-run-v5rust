template<class T>
struct box {
  template<class U>
  box(box<U> const&);
};

box<int>* p;
int main() { return 0; }
