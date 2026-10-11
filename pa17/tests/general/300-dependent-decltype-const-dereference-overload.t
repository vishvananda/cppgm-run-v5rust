template<class T> struct box { T& operator*(); T const& operator*() const; };
struct deref {
  template<class R> auto operator()(R const& r) const -> decltype(*r) {
    return *r;
  }
};
int main() { box<int> b; return deref()(b); }
