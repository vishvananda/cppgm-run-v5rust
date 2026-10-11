template<class T1, class T2>
struct pair {
  using first_type = T1;
  using second_type = T2;
  T1 first;
  T2 second;

  pair& operator=(const pair& p) {
    first = p.first;
    second = p.second;
    return *this;
  }
};

template<class U1, class U2>
struct holder {
  pair<U1, U2> value;
};
