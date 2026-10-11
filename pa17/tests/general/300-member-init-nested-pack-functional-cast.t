template<unsigned long... I> struct indices {};
template<unsigned long I, class... T> struct element;
template<class A, class B> struct element<0, A, B> { typedef A type; };
template<class A, class B> struct element<1, A, B> { typedef B type; };

template<class... T>
struct wrapper {
  wrapper(int, const T&...) {}
  template<unsigned long... I, class... Args>
  wrapper(indices<I...>, Args&&...) :
    wrapper(0, typename element<sizeof...(Args) + I, T...>::type()...) {}
  wrapper() : wrapper(indices<0, 1>{}) {}
};

wrapper<int, char> value;
int main() { return 0; }
