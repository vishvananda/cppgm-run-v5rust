// N3485 focus: 14.8.2 [temp.deduct] substitution proceeds in lexical order.
template<class T> struct hard { static_assert(sizeof(T) == 0, "hard"); typedef int type; };
template<class T> using result = typename hard<T>::type;
template<class T> struct gate {};
template<class T> result<T> choose(T, typename gate<T>::type = {});
int choose(...);
int main() { choose(0, 0); }
