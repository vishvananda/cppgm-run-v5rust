// expect reject
template<class T> int value = 0;
template<class U> int value<U*> = missing;
