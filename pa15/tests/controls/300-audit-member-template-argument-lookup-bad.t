// expect reject
template<class T> struct A {
private:
    using I = int;
public:
    template<class U> int f(U) { return 1; }
};
template<> template<> int A<int>::f<I>(int) { return 3; }
