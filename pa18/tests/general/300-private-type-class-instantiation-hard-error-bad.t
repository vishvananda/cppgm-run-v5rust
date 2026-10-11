// A deduction's class-instantiation side effect is outside its immediate context.
class Hidden { typedef int type; };
template<class T> struct Body { typedef typename T::type type; };
template<class T> int pick(typename Body<T>::type*) { return 1; }
template<class T> int pick(...) { return 2; }
int main() { return pick<Hidden>(0); }
