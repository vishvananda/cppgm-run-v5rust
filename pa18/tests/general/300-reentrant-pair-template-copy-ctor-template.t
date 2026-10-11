template<class T1, class T2>
struct pair {
  template<class U1, class U2>
  pair(pair<U1, U2> const&);
};

pair<int, int>* p;
int main() { return 0; }
