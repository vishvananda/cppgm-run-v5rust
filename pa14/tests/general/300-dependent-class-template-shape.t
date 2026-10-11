template<class T1, class T2>
struct pair {
  using first_type = T1;
  using second_type = T2;
  T1 first;
  T2 second;
};

template<class T1, class T2, class U1, class U2>
inline bool eq(const pair<T1, T2>& x, const pair<U1, U2>& y) {
  return x.first == y.first && x.second == y.second;
}
