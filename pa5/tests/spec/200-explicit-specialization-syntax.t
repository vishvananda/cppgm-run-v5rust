// N3485 focus: 14.7.3 [temp.expl.spec] explicit specialization syntax
template<class T> struct Trait { static const int value = 0; };
template<> struct Trait<int> { static const int value = 1; };

template<class T> int f(T);
template<> int f<int>(int) { return 1; }
