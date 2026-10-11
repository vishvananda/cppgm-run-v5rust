// N3485 focus: 14.8.2 [temp.deduct], a value-dependent member-template call
// in defaulted non-type SFINAE is not evaluated before function deduction.
template<bool> struct enable_if {};
template<> struct enable_if<true> { typedef int type; };
template<bool B> using enable_if_t = typename enable_if<B>::type;

template<class, class> struct same { static const bool value = false; };
template<class T> struct same<T, T> { static const bool value = true; };

template<class T> struct target {
  struct check {
    template<class U> static constexpr bool enabled() { return same<T, U>::value; }
  };
  template<class U, enable_if_t<check::template enabled<U>()> = 0> target(U) {}
};

int main() { target<int> value(0); }
