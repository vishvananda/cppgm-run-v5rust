// expect exit 0
namespace associated {
template<class T> struct Cursor {
  T* pointer;
  T* base() const { return pointer; }
};
}
namespace adapter {
template<class T> struct Reverse {
  T cursor;
  T base() const { return cursor; }
};
template<class L, class R>
auto operator-(const Reverse<L>& left, const Reverse<R>& right)
  -> decltype(right.base() - left.base()) {
  return right.base() - left.base();
}
}
namespace associated {
template<class T>
long operator-(const Cursor<T>& left, const Cursor<T>& right) {
  return left.base() - right.base();
}
}
int main() {
  int values[4];
  adapter::Reverse<associated::Cursor<int> > first = {{values}};
  adapter::Reverse<associated::Cursor<int> > last = {{values + 3}};
  return (first - last) != 3;
}
