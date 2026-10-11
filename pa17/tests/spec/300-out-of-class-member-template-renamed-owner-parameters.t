namespace n { template<class A> struct D {}; }

template<class A> struct Q {
  template<class B> void f(const n::D<B>&, A&);
};

template<class X> template<class Y>
void Q<X>::f(const n::D<Y>&, X&) {}

void g(Q<int>& q, n::D<char>& d, int& i) { q.f(d, i); }
