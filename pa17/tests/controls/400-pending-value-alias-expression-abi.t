// expect exit 0
template<bool, class T> struct Enable {};
template<class T> struct Enable<true, T> { typedef T type; };
template<bool B, class T> using Enabled = typename Enable<B,T>::type;
template<bool B> struct Constraints {
  template<class U> static constexpr bool accepts() { return B; }
};
template<bool B> using Check = Constraints<B>;
template<bool B, class U> using Implicit = Enabled<Check<B>::template accepts<U>(), bool>;
template<class U> constexpr bool excluded() { return false; }
template<class U, Implicit<!excluded<U>(),U> = true>
int f(U value) { return value; }
int main() { return f(3)-3; }
