template<class T> struct actor { typename T::missing operator()() const; };
template<class T> struct function {
  actor<T> operator()() const;
  template<class A> int operator()(A const&) const { return 0; }
};
struct operation {};
int main() { return function<operation>()(1); }
