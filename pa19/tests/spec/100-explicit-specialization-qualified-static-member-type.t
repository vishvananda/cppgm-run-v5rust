// VALIDATION: compile-pass
// N3485 focus: 14.7.3 [temp.expl.spec]

template<class> struct box;
template<> struct box<char> { typedef int type; static type value; };
box<char>::type box<char>::value = 0;

int main() {}
