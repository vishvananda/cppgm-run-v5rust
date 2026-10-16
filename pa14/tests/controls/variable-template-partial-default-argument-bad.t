// expect reject
template<class T> int value = 0;
template<class U = int> int value<U*> = 1;
