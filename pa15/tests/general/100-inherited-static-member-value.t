// HHC-116
template<class T, T v> struct integral_constant { static const T value = v; };
template<class T> struct is_const : integral_constant<bool, false> {};
static_assert(!is_const<char>::value, "");
int main(){return 0;}
