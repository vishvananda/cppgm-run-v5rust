// HHC-170
template<class T>
struct pair {
  T first;
  T second;
};

pair<int> make(int a, int b) { return pair<int>(a, b); }
