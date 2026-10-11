// VALIDATION: compile-pass
template<bool> struct enable {};
template<> struct enable<true> { typedef void type; };
template<class T> struct first { typedef T type; };
template<class... T> using all = typename first<T...>::type;
template<class...> struct box {};
template<class O, class T, class E = void> struct can_output;
template<class O, class... T,
         class = typename enable<all<can_output<O, T>...>::value>::type>
void output(O&, box<T...> const&);
template<class O, class T, class E>
struct can_output { static const bool value = false; };
template<class O, class T>
struct can_output<O, T, decltype(output(*(O*)0, *(T*)0))>
{ static const bool value = true; };
int main() { return can_output<int, box<int>>::value; }
