// N3485 focus: 14.8.2.4 [temp.deduct.partial], 14.5.7 [temp.alias]
// Expand instantiated member aliases before comparing function templates.

struct model {
  template<class T> using iterator = T *;
  template<class T> using const_iterator = const T *;
};
template<class T> struct segment {
  template<class U> using iterator = typename T::template iterator<U>;
  template<class U> using const_iterator = typename T::template const_iterator<U>;
  template<class U> int insert(const_iterator<U>);
  template<class U> long insert(iterator<U>);
};
int main() {
  segment<model> s;
  const int *p = 0;
  return sizeof(s.insert(p)) == sizeof(int) ? 0 : 1;
}
