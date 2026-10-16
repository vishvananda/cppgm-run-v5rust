// expect reject
template<class T> int value = 0;
template<class U> int value<typename U::type> = 1;
